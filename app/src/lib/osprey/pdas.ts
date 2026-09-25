import { PublicKey } from "@solana/web3.js";

import {
  MARKET_SEED,
  OSPREY_PROGRAM_ID,
  POSITION_SEED,
  RISK_SNAPSHOT_SEED,
  VAULT_SEED,
} from "@/lib/solana/constants";

export function deriveMarketPda(collateralMint: PublicKey) {
  return PublicKey.findProgramAddressSync(
    [MARKET_SEED, collateralMint.toBuffer()],
    OSPREY_PROGRAM_ID
  );
}

export function derivePositionPda(market: PublicKey, owner: PublicKey) {
  return PublicKey.findProgramAddressSync(
    [POSITION_SEED, market.toBuffer(), owner.toBuffer()],
    OSPREY_PROGRAM_ID
  );
}

export function deriveVaultAuthorityPda(market: PublicKey) {
  return PublicKey.findProgramAddressSync(
    [VAULT_SEED, market.toBuffer()],
    OSPREY_PROGRAM_ID
  );
}

export function deriveRiskSnapshotPda(market: PublicKey) {
  return PublicKey.findProgramAddressSync(
    [RISK_SNAPSHOT_SEED, market.toBuffer()],
    OSPREY_PROGRAM_ID
  );
}
