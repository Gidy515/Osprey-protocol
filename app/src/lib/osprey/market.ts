import { BorshAccountsCoder, Idl } from "@coral-xyz/anchor";
import {
  Connection,
  PublicKey,
} from "@solana/web3.js";

import idl from "@/lib/anchor/idl.json";
import { DEMO_ANTHROPIC_MINT } from "@/lib/solana/constants";
import {
  deriveMarketPda,
  deriveRiskSnapshotPda,
} from "@/lib/osprey/pdas";

export type OspreyMarket = {
  address: PublicKey;
  collateralMint: PublicKey;
  debtMint: PublicKey;
  dlmmPool: PublicKey;
  authority: PublicKey;

  collateralPriceUsdc: bigint;

  maxLtvBps: number;
  minLtvBps: number;
  liquidationThresholdBps: number;
  liquidationBonusBps: number;
  maxLiquidationBps: number;

  referenceLiquidationSizeUsdc: bigint;
  issuerRiskCeilingBps: number;
  transferFeeBps: number;
};

export type LiquidityRiskSnapshot = {
  address: PublicKey;
  market: PublicKey;
  quoteCollateralIn: bigint;
  quoteUsdcOut: bigint;
  observedSlot: bigint;
};

export type RiskMetrics = {
  recoveryBps: number;
  executionLtvBps: number;
  effectiveLtvBps: number;
};

function toBigInt(value: unknown): bigint {
  if (
    typeof value === "object" &&
    value !== null &&
    "toString" in value
  ) {
    return BigInt(value.toString());
  }

  return BigInt(String(value));
}

export async function fetchDemoMarket(
  connection: Connection
): Promise<{
  market: OspreyMarket;
  snapshot: LiquidityRiskSnapshot;
  risk: RiskMetrics;
}> {
  const [marketAddress] =
    deriveMarketPda(DEMO_ANTHROPIC_MINT);

  const [snapshotAddress] =
    deriveRiskSnapshotPda(marketAddress);

  const [marketInfo, snapshotInfo] =
    await Promise.all([
      connection.getAccountInfo(marketAddress),
      connection.getAccountInfo(snapshotAddress),
    ]);

  if (!marketInfo) {
    throw new Error("Osprey market account not found.");
  }

  if (!snapshotInfo) {
    throw new Error(
      "Osprey liquidity-risk snapshot not found."
    );
  }

  const coder = new BorshAccountsCoder(idl as Idl);

  const marketDecoded = coder.decode(
    "MarketConfig",
    marketInfo.data
  ) as any;

  const snapshotDecoded = coder.decode(
    "LiquidityRiskSnapshot",
    snapshotInfo.data
  ) as any;

  const market: OspreyMarket = {
    address: marketAddress,

    collateralMint:
      marketDecoded.collateral_mint,

    debtMint:
      marketDecoded.debt_mint,

    dlmmPool:
      marketDecoded.dlmm_pool,

    authority:
      marketDecoded.authority,

    collateralPriceUsdc: toBigInt(
      marketDecoded.collateral_price_usdc
    ),

    maxLtvBps: Number(
      marketDecoded.max_ltv_bps
    ),

    minLtvBps: Number(
      marketDecoded.min_ltv_bps
    ),

    liquidationThresholdBps: Number(
      marketDecoded.liquidation_threshold_bps
    ),

    liquidationBonusBps: Number(
      marketDecoded.liquidation_bonus_bps
    ),

    maxLiquidationBps: Number(
      marketDecoded.max_liquidation_bps
    ),

    referenceLiquidationSizeUsdc: toBigInt(
      marketDecoded.reference_liquidation_size_usdc
    ),

    issuerRiskCeilingBps: Number(
      marketDecoded.issuer_risk_ceiling_bps
    ),

    transferFeeBps: Number(
      marketDecoded.transfer_fee_bps
    ),
  };

  const snapshot: LiquidityRiskSnapshot = {
    address: snapshotAddress,

    market:
      snapshotDecoded.market,

    quoteCollateralIn: toBigInt(
      snapshotDecoded.quote_collateral_in
    ),

    quoteUsdcOut: toBigInt(
      snapshotDecoded.quote_usdc_out
    ),

    observedSlot: toBigInt(
      snapshotDecoded.observed_slot
    ),
  };

  const reference =
    market.referenceLiquidationSizeUsdc;

  if (reference <= BigInt(0)) {
    throw new Error(
      "Invalid reference liquidation size."
    );
  }

  const cappedOutput =
    snapshot.quoteUsdcOut > reference
      ? reference
      : snapshot.quoteUsdcOut;

  const BPS = BigInt(10_000);

  const recoveryBpsBig =
    (cappedOutput * BPS) / reference;

  let executionLtvBig =
    (BigInt(market.maxLtvBps) *
      recoveryBpsBig) /
    BPS;

  if (
    executionLtvBig <
    BigInt(market.minLtvBps)
  ) {
    executionLtvBig =
      BigInt(market.minLtvBps);
  }

  if (
    executionLtvBig >
    BigInt(market.maxLtvBps)
  ) {
    executionLtvBig =
      BigInt(market.maxLtvBps);
  }

  const effectiveLtvBig =
    executionLtvBig <
    BigInt(market.issuerRiskCeilingBps)
      ? executionLtvBig
      : BigInt(market.issuerRiskCeilingBps);

  return {
    market,
    snapshot,

    risk: {
      recoveryBps:
        Number(recoveryBpsBig),

      executionLtvBps:
        Number(executionLtvBig),

      effectiveLtvBps:
        Number(effectiveLtvBig),
    },
  };
}
