import { BorshAccountsCoder, Idl } from "@coral-xyz/anchor";
import { Connection, PublicKey } from "@solana/web3.js";

import idl from "@/lib/anchor/idl.json";
import { derivePositionPda } from "@/lib/osprey/pdas";

export type OspreyPosition = {
  address: PublicKey;
  owner: PublicKey;
  market: PublicKey;
  collateralAmount: bigint;
  debtAmount: bigint;
};

function toBigInt(value: unknown): bigint {
  if (typeof value === "object" && value !== null && "toString" in value) {
    return BigInt(value.toString());
  }

  return BigInt(String(value));
}

export async function fetchPosition(
  connection: Connection,
  market: PublicKey,
  owner: PublicKey
): Promise<OspreyPosition | null> {
  const [positionAddress] = derivePositionPda(market, owner);

  const accountInfo = await connection.getAccountInfo(positionAddress);

  if (!accountInfo) {
    return null;
  }

  const coder = new BorshAccountsCoder(idl as Idl);

  const decoded = coder.decode("Position", accountInfo.data) as any;

  return {
    address: positionAddress,
    owner: decoded.owner,
    market: decoded.market,

    collateralAmount: toBigInt(decoded.collateral_amount),

    debtAmount: toBigInt(decoded.debt_amount),
  };
}
