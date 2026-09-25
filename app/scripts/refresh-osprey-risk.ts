import fs from "node:fs";
import os from "node:os";
import path from "node:path";

import * as anchor from "@coral-xyz/anchor";
import {
  AnchorProvider,
  BN,
  Program,
  Wallet,
} from "@coral-xyz/anchor";
import DLMM from "@meteora-ag/dlmm";
import {
  Connection,
  Keypair,
  PublicKey,
  SystemProgram,
} from "@solana/web3.js";

import idl from "../src/lib/anchor/idl.json";

const RPC_URL = "https://api.devnet.solana.com";

const PROGRAM_ID = new PublicKey(
  "jAUJs14M4WiAvrgRgZiKQ2nunkWMrqqM7sGZotmWwss",
);

const COLLATERAL_MINT = new PublicKey(
  "ArzQGNtfXXQSLcdtVrJPu1CuJZHTiejL55hfum35FsKv",
);

const DLMM_POOL = new PublicKey(
  "53vdaEoUXhhTXGM9odCYVdvGYzLJnSMXovX5MX9E3xnU",
);

// 5 Demo ANTH, 9 decimals.
const REFERENCE_COLLATERAL_AMOUNT = new BN("5000000000");

const MARKET_SEED = Buffer.from("market");
const RISK_SNAPSHOT_SEED = Buffer.from("risk-snapshot");

function loadAuthority(): Keypair {
  const keypairPath = path.join(
    os.homedir(),
    ".config",
    "solana",
    "id.json",
  );

  const secret = JSON.parse(
    fs.readFileSync(keypairPath, "utf8"),
  ) as number[];

  return Keypair.fromSecretKey(Uint8Array.from(secret));
}

async function main() {
  console.log("\n====================================");
  console.log("OSPREY RISK SNAPSHOT REFRESH");
  console.log("====================================\n");

  const connection = new Connection(RPC_URL, "confirmed");
  const authority = loadAuthority();

  const wallet = new Wallet(authority);

  const provider = new AnchorProvider(connection, wallet, {
    commitment: "confirmed",
    preflightCommitment: "confirmed",
  });

  anchor.setProvider(provider);

  const program = new Program(
    idl as anchor.Idl,
    provider,
  );

  if (!program.programId.equals(PROGRAM_ID)) {
    throw new Error(
      `IDL program ID mismatch.\nExpected: ${PROGRAM_ID.toBase58()}\nActual:   ${program.programId.toBase58()}`,
    );
  }

  console.log("Authority:");
  console.log(authority.publicKey.toBase58());

  console.log("\nProgram:");
  console.log(program.programId.toBase58());

  /*
   * ---------------------------------------------------------
   * Derive Osprey PDAs
   * ---------------------------------------------------------
   */

  const [market] = PublicKey.findProgramAddressSync(
    [MARKET_SEED, COLLATERAL_MINT.toBuffer()],
    PROGRAM_ID,
  );

  const [riskSnapshot] = PublicKey.findProgramAddressSync(
    [RISK_SNAPSHOT_SEED, market.toBuffer()],
    PROGRAM_ID,
  );

  console.log("\nMarket:");
  console.log(market.toBase58());

  console.log("\nRisk snapshot:");
  console.log(riskSnapshot.toBase58());

  /*
   * ---------------------------------------------------------
   * 1. Fetch live Meteora quote
   * ---------------------------------------------------------
   */

  console.log("\n=== 1. Fetch Live Meteora Quote ===");

  const dlmm = await DLMM.create(
    connection,
    DLMM_POOL,
    {
      cluster: "devnet",
    },
  );

  const activeBin = await dlmm.getActiveBin();

  console.log("\nDLMM pool:");
  console.log(DLMM_POOL.toBase58());

  console.log("\nActive bin:");
  console.log(activeBin.binId);

  console.log("\nCurrent human price:");
  console.log(activeBin.pricePerToken);

  // Demo ANTH is token X and Demo USDC is token Y.
  // Therefore liquidation is X -> Y.
  const swapForY = true;

  const binArrays = await dlmm.getBinArrayForSwap(
    swapForY,
  );

  const quote = dlmm.swapQuote(
    REFERENCE_COLLATERAL_AMOUNT,
    swapForY,
    new BN(100),
    binArrays,
  );

  const quoteCollateralIn =
    REFERENCE_COLLATERAL_AMOUNT;

  const quoteUsdcOut = quote.outAmount;

  console.log("\nReference collateral input:");
  console.log(
    `${Number(quoteCollateralIn.toString()) / 1_000_000_000} Demo ANTH`,
  );

  console.log("\nExpected output raw:");
  console.log(quoteUsdcOut.toString());

  console.log("\nExpected Demo USDC:");
  console.log(
    Number(quoteUsdcOut.toString()) / 1_000_000,
  );

  console.log("\nMinimum output raw:");
  console.log(quote.minOutAmount.toString());

  console.log("\nSDK price impact:");
  console.log(quote.priceImpact.toString());

  /*
   * ---------------------------------------------------------
   * 2. Read Osprey market configuration
   * ---------------------------------------------------------
   */

  console.log("\n=== 2. Read Osprey Market ===");

  const marketAccount = await (
    program.account as any
  ).marketConfig.fetch(market);

  console.log(
    "\nReference liquidation size:",
    marketAccount.referenceLiquidationSizeUsdc.toString(),
  );

  console.log(
    "Maximum LTV:",
    `${marketAccount.maxLtvBps / 100}%`,
  );

  console.log(
    "Minimum LTV:",
    `${marketAccount.minLtvBps / 100}%`,
  );

  console.log(
    "Issuer ceiling:",
    `${marketAccount.issuerRiskCeilingBps / 100}%`,
  );

  /*
   * ---------------------------------------------------------
   * 3. Mirror Osprey risk calculation locally
   * ---------------------------------------------------------
   */

  console.log("\n=== 3. Osprey Risk Calculation ===");

  const referenceSize = BigInt(
    marketAccount.referenceLiquidationSizeUsdc.toString(),
  );

  const output = BigInt(
    quoteUsdcOut.toString(),
  );

  const cappedOutput =
    output > referenceSize
      ? referenceSize
      : output;

  const denominator = BigInt(10_000);

  const recoveryBps =
    (cappedOutput * denominator) /
    referenceSize;

  let executionLtvBps =
    (BigInt(marketAccount.maxLtvBps) *
      recoveryBps) /
    denominator;

  const minLtv = BigInt(
    marketAccount.minLtvBps,
  );

  const maxLtv = BigInt(
    marketAccount.maxLtvBps,
  );

  if (executionLtvBps < minLtv) {
    executionLtvBps = minLtv;
  }

  if (executionLtvBps > maxLtv) {
    executionLtvBps = maxLtv;
  }

  const issuerCeiling = BigInt(
    marketAccount.issuerRiskCeilingBps,
  );

  const effectiveLtvBps =
    executionLtvBps < issuerCeiling
      ? executionLtvBps
      : issuerCeiling;

  console.log(
    "Recovery:",
    `${Number(recoveryBps) / 100}%`,
  );

  console.log(
    "Execution LTV:",
    `${Number(executionLtvBps) / 100}%`,
  );

  console.log(
    "Issuer ceiling:",
    `${Number(issuerCeiling) / 100}%`,
  );

  console.log(
    "Effective LTV:",
    `${Number(effectiveLtvBps) / 100}%`,
  );

  /*
   * ---------------------------------------------------------
   * 4. Submit fresh snapshot
   * ---------------------------------------------------------
   */

  console.log("\n=== 4. Update Liquidity Risk ===");

  console.log(
    "\nquote_collateral_in:",
    quoteCollateralIn.toString(),
  );

  console.log(
    "quote_usdc_out:",
    quoteUsdcOut.toString(),
  );

  const signature = await program.methods
    .updateLiquidityRisk(
      quoteCollateralIn,
      quoteUsdcOut,
    )
    .accounts({
      authority: authority.publicKey,
      market,
      riskSnapshot,
      systemProgram: SystemProgram.programId,
    })
    .rpc();

  console.log("\nRisk snapshot transaction:");
  console.log(signature);

  /*
   * ---------------------------------------------------------
   * 5. Fetch snapshot back from chain
   * ---------------------------------------------------------
   */

  console.log("\n=== 5. Verify On-Chain Snapshot ===");

  const snapshotAccount = await (
    program.account as any
  ).liquidityRiskSnapshot.fetch(
    riskSnapshot,
  );

  console.log(
    "\nSnapshot collateral input:",
    snapshotAccount.quoteCollateralIn.toString(),
  );

  console.log(
    "Snapshot USDC output:",
    snapshotAccount.quoteUsdcOut.toString(),
  );

  console.log(
    "Snapshot observed slot:",
    snapshotAccount.observedSlot.toString(),
  );

  const currentSlot =
    await connection.getSlot("confirmed");

  const observedSlot = Number(
    snapshotAccount.observedSlot.toString(),
  );

  const age = currentSlot - observedSlot;

  console.log(
    "Current confirmed slot:",
    currentSlot,
  );

  console.log(
    "Snapshot age:",
    `${age} slots`,
  );

  if (age > 300) {
    throw new Error(
      `Snapshot is already stale: ${age} slots old.`,
    );
  }

  console.log("\n====================================");
  console.log("OSPREY RISK SNAPSHOT IS FRESH");
  console.log("====================================");

  console.log(
    `\nEffective LTV: ${Number(effectiveLtvBps) / 100}%`,
  );

  console.log(
    `Snapshot age: ${age} slots`,
  );

  console.log(
    "\nBorrow transaction can now be submitted.",
  );
}

main().catch((error) => {
  console.error(
    "\nOsprey risk refresh failed:\n",
  );

  console.error(error);

  process.exit(1);
});
