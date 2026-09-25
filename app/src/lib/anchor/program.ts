import { AnchorProvider, Idl, Program } from "@coral-xyz/anchor";
import { Connection } from "@solana/web3.js";
import type { AnchorWallet } from "@solana/wallet-adapter-react";

import idl from "./idl.json";

export function getOspreyProgram(
  connection: Connection,
  wallet: AnchorWallet
) {
  const provider = new AnchorProvider(
    connection,
    wallet,
    {
      commitment: "confirmed",
      preflightCommitment: "confirmed",
    }
  );

  return new Program(idl as Idl, provider);
}
