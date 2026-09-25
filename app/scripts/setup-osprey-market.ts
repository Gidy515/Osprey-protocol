import fs from "node:fs";
import os from "node:os";
import path from "node:path";

import * as anchor from "@coral-xyz/anchor";
import { AnchorProvider, BN, Program, Wallet } from "@coral-xyz/anchor";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_2022_PROGRAM_ID,
  createAssociatedTokenAccountInstruction,
  createTransferCheckedInstruction,
  getAccount,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";
import {
  Connection,
  Keypair,
  PublicKey,
  SystemProgram,
  Transaction,
  sendAndConfirmTransaction,
} from "@solana/web3.js";

import idl from "../src/lib/anchor/idl.json";

const RPC_URL = "https://api.devnet.solana.com";

const PROGRAM_ID = new PublicKey("jAUJs14M4WiAvrgRgZiKQ2nunkWMrqqM7sGZotmWwss");

const COLLATERAL_MINT = new PublicKey(
  "ArzQGNtfXXQSLcdtVrJPu1CuJZHTiejL55hfum35FsKv"
);

const DEBT_MINT = new PublicKey("9nPD667QBEV9rzmySdAqttPM5aTQWkjFH3R6JLhhF734");

const DLMM_POOL = new PublicKey("53vdaEoUXhhTXGM9odCYVdvGYzLJnSMXovX5MX9E3xnU");

// $1,000 with 6-decimal USDC denomination.
const COLLATERAL_PRICE_USDC = new BN("1000000000");

// Real Meteora quote we just measured.
const QUOTE_COLLATERAL_IN = new BN("5000000000");
const QUOTE_USDC_OUT = new BN("4495562559");

// Seed Osprey's borrowing vault with 20,000 Demo USDC.
const DEBT_VAULT_FUNDING = BigInt("20000000000");

const MARKET_SEED = Buffer.from("market");
const VAULT_SEED = Buffer.from("vault");
const RISK_SNAPSHOT_SEED = Buffer.from("risk-snapshot");

function loadAuthority(): Keypair {
  const keypairPath = path.join(os.homedir(), ".config", "solana", "id.json");

  const secret = JSON.parse(fs.readFileSync(keypairPath, "utf8")) as number[];

  return Keypair.fromSecretKey(Uint8Array.from(secret));
}

async function main() {
  const connection = new Connection(RPC_URL, "confirmed");
  const authority = loadAuthority();

  const wallet = new Wallet(authority);

  const provider = new AnchorProvider(connection, wallet, {
    commitment: "confirmed",
    preflightCommitment: "confirmed",
  });

  anchor.setProvider(provider);

  const program = new Program(idl as anchor.Idl, provider);

  if (!program.programId.equals(PROGRAM_ID)) {
    throw new Error(
      `IDL program ID mismatch.
Expected: ${PROGRAM_ID.toBase58()}
Actual:   ${program.programId.toBase58()}`
    );
  }

  console.log("\n=== Osprey Devnet Market Setup ===\n");

  console.log("Authority:");
  console.log(authority.publicKey.toBase58());

  console.log("\nProgram:");
  console.log(program.programId.toBase58());

  /*
   * ---------------------------------------------------------
   * Derive protocol PDAs
   * ---------------------------------------------------------
   */

  const [market] = PublicKey.findProgramAddressSync(
    [MARKET_SEED, COLLATERAL_MINT.toBuffer()],
    PROGRAM_ID
  );

  const [vaultAuthority] = PublicKey.findProgramAddressSync(
    [VAULT_SEED, market.toBuffer()],
    PROGRAM_ID
  );

  const [riskSnapshot] = PublicKey.findProgramAddressSync(
    [RISK_SNAPSHOT_SEED, market.toBuffer()],
    PROGRAM_ID
  );

  console.log("\n=== Derived Addresses ===");

  console.log("Market:");
  console.log(market.toBase58());

  console.log("\nVault authority:");
  console.log(vaultAuthority.toBase58());

  console.log("\nRisk snapshot:");
  console.log(riskSnapshot.toBase58());

  /*
   * ---------------------------------------------------------
   * 1. Initialize Market
   * ---------------------------------------------------------
   */

  console.log("\n=== 1. Initialize Market ===");

  const existingMarket = await connection.getAccountInfo(market);

  if (existingMarket) {
    console.log("Market already exists. Skipping initialize.");
  } else {
    const signature = await program.methods
      .initialize({
        collateralMint: COLLATERAL_MINT,
        debtMint: DEBT_MINT,
        dlmmPool: DLMM_POOL,
        collateralPriceUsdc: COLLATERAL_PRICE_USDC,
      })
      .accounts({
        market,
        authority: authority.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    console.log("Initialize transaction:");
    console.log(signature);
  }

  /*
   * ---------------------------------------------------------
   * 2. Create protocol Demo USDC debt vault
   * ---------------------------------------------------------
   *
   * Owner = vaultAuthority PDA.
   *
   * Because the owner is a PDA, allowOwnerOffCurve must be true.
   */

  console.log("\n=== 2. Debt Vault ===");

  const debtVault = getAssociatedTokenAddressSync(
    DEBT_MINT,
    vaultAuthority,
    true,
    TOKEN_2022_PROGRAM_ID,
    ASSOCIATED_TOKEN_PROGRAM_ID
  );

  console.log("Debt vault:");
  console.log(debtVault.toBase58());

  const existingDebtVault = await connection.getAccountInfo(debtVault);

  if (!existingDebtVault) {
    console.log("\nCreating vault-authority Demo USDC ATA...");

    const createAtaIx = createAssociatedTokenAccountInstruction(
      authority.publicKey,
      debtVault,
      vaultAuthority,
      DEBT_MINT,
      TOKEN_2022_PROGRAM_ID,
      ASSOCIATED_TOKEN_PROGRAM_ID
    );

    const tx = new Transaction().add(createAtaIx);

    const signature = await sendAndConfirmTransaction(
      connection,
      tx,
      [authority],
      {
        commitment: "confirmed",
      }
    );

    console.log("Create debt vault transaction:");
    console.log(signature);
  } else {
    console.log("Debt vault already exists. Skipping creation.");
  }

  /*
   * ---------------------------------------------------------
   * 3. Fund debt vault
   * ---------------------------------------------------------
   */

  console.log("\n=== 3. Fund Debt Vault ===");

  const authorityDebtAccount = getAssociatedTokenAddressSync(
    DEBT_MINT,
    authority.publicKey,
    false,
    TOKEN_2022_PROGRAM_ID,
    ASSOCIATED_TOKEN_PROGRAM_ID
  );

  console.log("Authority Demo USDC account:");
  console.log(authorityDebtAccount.toBase58());

  const debtVaultStateBefore = await getAccount(
    connection,
    debtVault,
    "confirmed",
    TOKEN_2022_PROGRAM_ID
  );

  console.log(
    "\nCurrent debt vault balance:",
    Number(debtVaultStateBefore.amount) / 1_000_000,
    "Demo USDC"
  );

  /*
   * Make this script rerunnable.
   *
   * Only fund the vault when its balance is currently zero.
   */
  if (debtVaultStateBefore.amount === BigInt(0)) {
    console.log("\nFunding with 20,000 Demo USDC...");

    const transferIx = createTransferCheckedInstruction(
      authorityDebtAccount,
      DEBT_MINT,
      debtVault,
      authority.publicKey,
      DEBT_VAULT_FUNDING,
      6,
      [],
      TOKEN_2022_PROGRAM_ID
    );

    const tx = new Transaction().add(transferIx);

    const signature = await sendAndConfirmTransaction(
      connection,
      tx,
      [authority],
      {
        commitment: "confirmed",
      }
    );

    console.log("Funding transaction:");
    console.log(signature);
  } else {
    console.log("Debt vault already funded. Skipping transfer.");
  }

  const debtVaultStateAfter = await getAccount(
    connection,
    debtVault,
    "confirmed",
    TOKEN_2022_PROGRAM_ID
  );

  console.log(
    "\nDebt vault balance:",
    Number(debtVaultStateAfter.amount) / 1_000_000,
    "Demo USDC"
  );

  /*
   * ---------------------------------------------------------
   * 4. Submit real Meteora liquidity snapshot
   * ---------------------------------------------------------
   */

  console.log("\n=== 4. Update Liquidity Risk ===");

  console.log("quote_collateral_in:", QUOTE_COLLATERAL_IN.toString());

  console.log("quote_usdc_out:", QUOTE_USDC_OUT.toString());

  const riskSignature = await program.methods
    .updateLiquidityRisk(QUOTE_COLLATERAL_IN, QUOTE_USDC_OUT)
    .accounts({
      authority: authority.publicKey,
      market,
      riskSnapshot,
      systemProgram: SystemProgram.programId,
    })
    .rpc();

  console.log("\nRisk snapshot transaction:");
  console.log(riskSignature);

  /*
   * ---------------------------------------------------------
   * 5. Fetch and verify on-chain state
   * ---------------------------------------------------------
   */

  console.log("\n=== 5. Verify On-Chain State ===");

  const marketAccount = await (program.account as any).marketConfig.fetch(
    market
  );

  const snapshotAccount = await (
    program.account as any
  ).liquidityRiskSnapshot.fetch(riskSnapshot);

  console.log("\nMarket collateral mint:");
  console.log(marketAccount.collateralMint.toBase58());

  console.log("\nMarket debt mint:");
  console.log(marketAccount.debtMint.toBase58());

  console.log("\nMarket DLMM pool:");
  console.log(marketAccount.dlmmPool.toBase58());

  console.log(
    "\nCollateral price raw:",
    marketAccount.collateralPriceUsdc.toString()
  );

  console.log("\nMax LTV:", `${marketAccount.maxLtvBps / 100}%`);

  console.log("Min LTV:", `${marketAccount.minLtvBps / 100}%`);

  console.log(
    "Issuer risk ceiling:",
    `${marketAccount.issuerRiskCeilingBps / 100}%`
  );

  console.log(
    "\nSnapshot collateral input:",
    snapshotAccount.quoteCollateralIn.toString()
  );

  console.log("Snapshot USDC output:", snapshotAccount.quoteUsdcOut.toString());

  console.log(
    "Snapshot observed slot:",
    snapshotAccount.observedSlot.toString()
  );

  /*
   * ---------------------------------------------------------
   * Calculate displayed effective LTV
   * ---------------------------------------------------------
   */

  const referenceSize = BigInt(
    marketAccount.referenceLiquidationSizeUsdc.toString()
  );

  const quoteOutput = BigInt(snapshotAccount.quoteUsdcOut.toString());

  const cappedOutput =
    quoteOutput > referenceSize ? referenceSize : quoteOutput;

  const denominator = BigInt(10_000);

  const recoveryBps = (cappedOutput * denominator) / referenceSize;

  let executionLtvBps =
    (BigInt(marketAccount.maxLtvBps) * recoveryBps) / denominator;

  const minLtv = BigInt(marketAccount.minLtvBps);

  const maxLtv = BigInt(marketAccount.maxLtvBps);

  if (executionLtvBps < minLtv) {
    executionLtvBps = minLtv;
  }

  if (executionLtvBps > maxLtv) {
    executionLtvBps = maxLtv;
  }

  const issuerCeiling = BigInt(marketAccount.issuerRiskCeilingBps);

  const effectiveLtvBps =
    executionLtvBps < issuerCeiling ? executionLtvBps : issuerCeiling;

  console.log("\n=== Risk Result ===");

  console.log("Recovery:", `${Number(recoveryBps) / 100}%`);

  console.log("Execution LTV:", `${Number(executionLtvBps) / 100}%`);

  console.log("Issuer ceiling:", `${Number(issuerCeiling) / 100}%`);

  console.log("Effective LTV:", `${Number(effectiveLtvBps) / 100}%`);

  /*
   * ---------------------------------------------------------
   * Save demo addresses
   * ---------------------------------------------------------
   */

  fs.mkdirSync(".demo", {
    recursive: true,
  });

  fs.writeFileSync(
    ".demo/osprey-market.json",
    JSON.stringify(
      {
        network: "devnet",

        programId: PROGRAM_ID.toBase58(),

        authority: authority.publicKey.toBase58(),

        collateralMint: COLLATERAL_MINT.toBase58(),

        debtMint: DEBT_MINT.toBase58(),

        dlmmPool: DLMM_POOL.toBase58(),

        market: market.toBase58(),

        vaultAuthority: vaultAuthority.toBase58(),

        debtVault: debtVault.toBase58(),

        riskSnapshot: riskSnapshot.toBase58(),

        collateralPriceUsdc: COLLATERAL_PRICE_USDC.toString(),

        quoteCollateralIn: QUOTE_COLLATERAL_IN.toString(),

        quoteUsdcOut: QUOTE_USDC_OUT.toString(),

        effectiveLtvBps: effectiveLtvBps.toString(),
      },
      null,
      2
    )
  );

  console.log("\nSaved .demo/osprey-market.json");

  console.log("\n====================================");

  console.log("OSPREY MARKET READY");

  console.log("====================================");
}

main().catch((error) => {
  console.error("\nOsprey market setup failed:\n");

  console.error(error);

  process.exit(1);
});
