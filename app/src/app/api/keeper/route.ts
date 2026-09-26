import * as anchor from "@coral-xyz/anchor";
import { AnchorProvider, BN, Program } from "@coral-xyz/anchor";
import DLMM from "@meteora-ag/dlmm";
import { Connection, Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import { NextRequest, NextResponse } from "next/server";

import idl from "@/lib/anchor/idl.json";
import {
  DEMO_ANTHROPIC_MINT,
  METEORA_DLMM_POOL,
  OSPREY_PROGRAM_ID,
  RISK_SNAPSHOT_SEED,
  MARKET_SEED,
  SOLANA_RPC_URL,
} from "@/lib/solana/constants";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";

const REFERENCE_COLLATERAL_AMOUNT = new BN("5000000000");

function getAuthority(): Keypair {
  const raw = process.env.OSPREY_AUTHORITY_SECRET;

  if (!raw) {
    throw new Error("OSPREY_AUTHORITY_SECRET is not configured.");
  }

  const secret = JSON.parse(raw) as number[];

  if (!Array.isArray(secret) || secret.length !== 64) {
    throw new Error("OSPREY_AUTHORITY_SECRET must be a 64-byte JSON array.");
  }

  return Keypair.fromSecretKey(Uint8Array.from(secret));
}

function authorized(request: NextRequest): boolean {
  const secret = process.env.KEEPER_SECRET;

  if (!secret) {
    throw new Error("KEEPER_SECRET is not configured.");
  }

  return request.headers.get("authorization") === `Bearer ${secret}`;
}

export async function GET(request: NextRequest) {
  try {
    if (!authorized(request)) {
      return NextResponse.json({ error: "Unauthorized." }, { status: 401 });
    }

    const connection = new Connection(SOLANA_RPC_URL, "confirmed");

    const authority = getAuthority();

    const wallet = {
      publicKey: authority.publicKey,

      async signTransaction<
        T extends anchor.web3.Transaction | anchor.web3.VersionedTransaction
      >(transaction: T): Promise<T> {
        if (transaction instanceof anchor.web3.VersionedTransaction) {
          transaction.sign([authority]);
        } else {
          transaction.partialSign(authority);
        }

        return transaction;
      },

      async signAllTransactions<
        T extends anchor.web3.Transaction | anchor.web3.VersionedTransaction
      >(transactions: T[]): Promise<T[]> {
        for (const transaction of transactions) {
          if (transaction instanceof anchor.web3.VersionedTransaction) {
            transaction.sign([authority]);
          } else {
            transaction.partialSign(authority);
          }
        }

        return transactions;
      },
    };

    const provider = new AnchorProvider(connection, wallet, {
      commitment: "confirmed",
      preflightCommitment: "confirmed",
    });

    anchor.setProvider(provider);

    const program = new Program(idl as anchor.Idl, provider);

    if (!program.programId.equals(OSPREY_PROGRAM_ID)) {
      throw new Error(
        `IDL program ID mismatch. Expected ${OSPREY_PROGRAM_ID.toBase58()}, got ${program.programId.toBase58()}.`
      );
    }

    const [market] = PublicKey.findProgramAddressSync(
      [MARKET_SEED, DEMO_ANTHROPIC_MINT.toBuffer()],
      OSPREY_PROGRAM_ID
    );

    const [riskSnapshot] = PublicKey.findProgramAddressSync(
      [RISK_SNAPSHOT_SEED, market.toBuffer()],
      OSPREY_PROGRAM_ID
    );

    /*
     * Fetch executable Meteora quote.
     */

    const dlmm = await DLMM.create(connection, METEORA_DLMM_POOL, {
      cluster: "devnet",
    });

    const swapForY = true;

    const binArrays = await dlmm.getBinArrayForSwap(swapForY);

    const quote = dlmm.swapQuote(
      REFERENCE_COLLATERAL_AMOUNT,
      swapForY,
      new BN(100),
      binArrays
    );

    const quoteCollateralIn = REFERENCE_COLLATERAL_AMOUNT;

    const quoteUsdcOut = quote.outAmount;

    /*
     * Publish the observation onchain.
     */

    const signature = await program.methods
      .updateLiquidityRisk(quoteCollateralIn, quoteUsdcOut)
      .accounts({
        authority: authority.publicKey,
        market,
        riskSnapshot,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    /*
     * Verify the resulting snapshot.
     */

    const snapshotAccount = await (
      program.account as any
    ).liquidityRiskSnapshot.fetch(riskSnapshot);

    const currentSlot = await connection.getSlot("confirmed");

    const observedSlot = Number(snapshotAccount.observedSlot.toString());

    const snapshotAge = currentSlot - observedSlot;

    if (snapshotAge > 300) {
      throw new Error(
        `Published snapshot is already stale: ${snapshotAge} slots.`
      );
    }

    return NextResponse.json({
      success: true,
      transaction: signature,
      market: market.toBase58(),
      riskSnapshot: riskSnapshot.toBase58(),
      quoteCollateralIn: quoteCollateralIn.toString(),
      quoteUsdcOut: quoteUsdcOut.toString(),
      observedSlot,
      currentSlot,
      snapshotAge,
    });
  } catch (error) {
    console.error("Osprey keeper failed:", error);

    return NextResponse.json(
      {
        success: false,
        error:
          error instanceof Error ? error.message : "Keeper refresh failed.",
      },
      {
        status: 500,
      }
    );
  }
}
