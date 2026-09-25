import DLMM from "@meteora-ag/dlmm";
import BN from "bn.js";
import { Connection, PublicKey } from "@solana/web3.js";

const RPC_URL = "https://api.devnet.solana.com";

const POOL = new PublicKey("53vdaEoUXhhTXGM9odCYVdvGYzLJnSMXovX5MX9E3xnU");

const COLLATERAL_MINT = new PublicKey(
  "ArzQGNtfXXQSLcdtVrJPu1CuJZHTiejL55hfum35FsKv"
);

// 5 ANTH × 10^9
const REFERENCE_COLLATERAL_AMOUNT = new BN("5000000000");

// Osprey's configured $5,000 reference liquidation size.
// USDC has 6 decimals.
const REFERENCE_USDC = new BN("5000000000");

const MAX_LTV_BPS = 6500;
const MIN_LTV_BPS = 3500;
const ISSUER_RISK_CEILING_BPS = 6000;

async function main() {
  const connection = new Connection(RPC_URL, "confirmed");

  console.log("\n=== Osprey Liquidity Risk Quote ===\n");

  const dlmm = await DLMM.create(connection, POOL, {
    cluster: "devnet",
  });

  const activeBin = await dlmm.getActiveBin();

  console.log("Pool:");
  console.log(POOL.toBase58());

  console.log("\nActive bin:");
  console.log(activeBin.binId);

  console.log("\nCurrent human price:");
  console.log(activeBin.pricePerToken);

  console.log("\nReference liquidation:");
  console.log("5 Demo ANTH (~$5,000)");

  /*
   * true because we're swapping token X -> token Y:
   *
   * Demo ANTH -> Demo USDC
   */
  const swapForY = true;

  const binArrays = await dlmm.getBinArrayForSwap(swapForY);

  const quote = dlmm.swapQuote(
    REFERENCE_COLLATERAL_AMOUNT,
    swapForY,
    new BN(100),
    binArrays
  );

  console.log("\n=== Meteora Quote ===");

  console.log("Input raw:");
  console.log(REFERENCE_COLLATERAL_AMOUNT.toString());

  console.log("\nExpected output raw:");
  console.log(quote.outAmount.toString());

  console.log("\nExpected Demo USDC:");
  console.log(Number(quote.outAmount.toString()) / 1_000_000);

  console.log("\nMinimum output raw:");
  console.log(quote.minOutAmount.toString());

  console.log("\nPrice impact:");
  console.log(quote.priceImpact.toString());

  /*
   * Mirror Osprey's on-chain risk math.
   *
   * recovery_bps =
   *     quote_usdc_out * 10_000
   *     / reference_liquidation_size_usdc
   *
   * execution_ltv =
   *     max_ltv * recovery_bps
   *     / 10_000
   *
   * Then clamp between min/max LTV.
   */

  const output = BigInt(quote.outAmount.toString());
  const reference = BigInt(REFERENCE_USDC.toString());

  const cappedOutput = output > reference ? reference : output;

  const BPS_DENOMINATOR = BigInt(10_000);

  const recoveryBps = (cappedOutput * BPS_DENOMINATOR) / reference;

  let executionLtvBps = (BigInt(MAX_LTV_BPS) * recoveryBps) / BPS_DENOMINATOR;

  if (executionLtvBps < BigInt(MIN_LTV_BPS)) {
    executionLtvBps = BigInt(MIN_LTV_BPS);
  }

  if (executionLtvBps > BigInt(MAX_LTV_BPS)) {
    executionLtvBps = BigInt(MAX_LTV_BPS);
  }

  const effectiveLtvBps =
    executionLtvBps < BigInt(ISSUER_RISK_CEILING_BPS)
      ? executionLtvBps
      : BigInt(ISSUER_RISK_CEILING_BPS);

  console.log("\n=== Osprey Risk Engine ===");

  console.log(`Recovery: ${Number(recoveryBps) / 100}%`);

  console.log(`Execution LTV: ${Number(executionLtvBps) / 100}%`);

  console.log(`Issuer ceiling: ${ISSUER_RISK_CEILING_BPS / 100}%`);

  console.log(`Effective LTV: ${Number(effectiveLtvBps) / 100}%`);

  console.log("\n=== Snapshot Values ===");

  console.log("quote_collateral_in:", REFERENCE_COLLATERAL_AMOUNT.toString());

  console.log("quote_usdc_out:", quote.outAmount.toString());
}

main().catch((error) => {
  console.error("\nQuote failed:\n");
  console.error(error);
  process.exit(1);
});
