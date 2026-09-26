import { NextRequest, NextResponse } from "next/server";
import {
  createAssociatedTokenAccountIdempotent,
  getAccount,
  mintTo,
  TOKEN_2022_PROGRAM_ID,
} from "@solana/spl-token";
import { Connection, Keypair, PublicKey } from "@solana/web3.js";

import {
  DEMO_ANTHROPIC_MINT,
  DEMO_USDC_MINT,
  SOLANA_RPC_URL,
} from "@/lib/solana/constants";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";

const TARGET_ANTH_BALANCE = BigInt("25000000000"); // 25 ANTH
const TARGET_USDC_BALANCE = BigInt("2500000000"); // 2,500 USDC

function getAuthority(): Keypair {
  const raw = process.env.OSPREY_AUTHORITY_SECRET;

  if (!raw) {
    throw new Error("OSPREY_AUTHORITY_SECRET is not configured.");
  }

  let secret: number[];

  try {
    secret = JSON.parse(raw) as number[];
  } catch {
    throw new Error("OSPREY_AUTHORITY_SECRET is not valid JSON.");
  }

  if (!Array.isArray(secret) || secret.length !== 64) {
    throw new Error("OSPREY_AUTHORITY_SECRET must be a 64-byte JSON array.");
  }

  return Keypair.fromSecretKey(Uint8Array.from(secret));
}

export async function POST(request: NextRequest) {
  try {
    const body = (await request.json()) as {
      wallet?: string;
    };

    if (!body.wallet) {
      return NextResponse.json(
        { error: "Wallet address is required." },
        { status: 400 }
      );
    }

    let recipient: PublicKey;

    try {
      recipient = new PublicKey(body.wallet);
    } catch {
      return NextResponse.json(
        { error: "Invalid Solana wallet address." },
        { status: 400 }
      );
    }

    if (!PublicKey.isOnCurve(recipient.toBytes())) {
      return NextResponse.json(
        { error: "Recipient must be a wallet address." },
        { status: 400 }
      );
    }

    const authority = getAuthority();

    const connection = new Connection(SOLANA_RPC_URL, "confirmed");

    const anthAta = await createAssociatedTokenAccountIdempotent(
      connection,
      authority,
      DEMO_ANTHROPIC_MINT,
      recipient,
      {},
      TOKEN_2022_PROGRAM_ID
    );

    const usdcAta = await createAssociatedTokenAccountIdempotent(
      connection,
      authority,
      DEMO_USDC_MINT,
      recipient,
      {},
      TOKEN_2022_PROGRAM_ID
    );
    const anthAccount = await getAccount(
      connection,
      anthAta,
      "confirmed",
      TOKEN_2022_PROGRAM_ID
    );

    const usdcAccount = await getAccount(
      connection,
      usdcAta,
      "confirmed",
      TOKEN_2022_PROGRAM_ID
    );

    const anthTopUp =
      anthAccount.amount < TARGET_ANTH_BALANCE
        ? TARGET_ANTH_BALANCE - anthAccount.amount
        : BigInt(0);

    const usdcTopUp =
      usdcAccount.amount < TARGET_USDC_BALANCE
        ? TARGET_USDC_BALANCE - usdcAccount.amount
        : BigInt(0);

    let anthSignature: string | null = null;
    let usdcSignature: string | null = null;

    if (anthTopUp > BigInt(0)) {
      anthSignature = await mintTo(
        connection,
        authority,
        DEMO_ANTHROPIC_MINT,
        anthAta,
        authority,
        anthTopUp,
        [],
        {},
        TOKEN_2022_PROGRAM_ID
      );
    }

    if (usdcTopUp > BigInt(0)) {
      usdcSignature = await mintTo(
        connection,
        authority,
        DEMO_USDC_MINT,
        usdcAta,
        authority,
        usdcTopUp,
        [],
        {},
        TOKEN_2022_PROGRAM_ID
      );
    }

    return NextResponse.json({
      success: true,
      anthTopUp: anthTopUp.toString(),
      usdcTopUp: usdcTopUp.toString(),
      alreadyFunded: anthTopUp === BigInt(0) && usdcTopUp === BigInt(0),
      anthSignature,
      usdcSignature,
    });
  } catch (error) {
    console.error("Osprey faucet failed:", error);

    return NextResponse.json(
      {
        error: error instanceof Error ? error.message : "Demo faucet failed.",
      },
      { status: 500 }
    );
  }
}
