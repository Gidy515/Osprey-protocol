import { BN } from "@coral-xyz/anchor";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_2022_PROGRAM_ID,
  createAssociatedTokenAccountInstruction,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";
import { SystemProgram, Transaction } from "@solana/web3.js";
import type { AnchorWallet } from "@solana/wallet-adapter-react";
import type { Connection } from "@solana/web3.js";

import { getOspreyProgram } from "@/lib/anchor/program";
import { DEMO_ANTHROPIC_MINT, DEMO_USDC_MINT } from "@/lib/solana/constants";
import {
  deriveMarketPda,
  derivePositionPda,
  deriveRiskSnapshotPda,
  deriveVaultAuthorityPda,
} from "@/lib/osprey/pdas";

async function ensureAssociatedTokenAccount(
  connection: Connection,
  wallet: AnchorWallet,
  mint: Parameters<typeof getAssociatedTokenAddressSync>[0]
) {
  const owner = wallet.publicKey;

  const ata = getAssociatedTokenAddressSync(
    mint,
    owner,
    false,
    TOKEN_2022_PROGRAM_ID,
    ASSOCIATED_TOKEN_PROGRAM_ID
  );

  const accountInfo = await connection.getAccountInfo(ata);

  if (accountInfo) {
    return ata;
  }

  const createAtaInstruction = createAssociatedTokenAccountInstruction(
    owner,
    ata,
    owner,
    mint,
    TOKEN_2022_PROGRAM_ID,
    ASSOCIATED_TOKEN_PROGRAM_ID
  );

  const transaction = new Transaction().add(createAtaInstruction);

  const { blockhash, lastValidBlockHeight } =
    await connection.getLatestBlockhash("confirmed");

  transaction.recentBlockhash = blockhash;
  transaction.feePayer = owner;

  const signedTransaction = await wallet.signTransaction(transaction);

  const signature = await connection.sendRawTransaction(
    signedTransaction.serialize(),
    {
      skipPreflight: false,
      preflightCommitment: "confirmed",
    }
  );

  await connection.confirmTransaction(
    {
      signature,
      blockhash,
      lastValidBlockHeight,
    },
    "confirmed"
  );

  return ata;
}

export async function depositCollateral(
  connection: Connection,
  wallet: AnchorWallet,
  amountRaw: bigint
) {
  const program = getOspreyProgram(connection, wallet);

  const owner = wallet.publicKey;

  const [market] = deriveMarketPda(DEMO_ANTHROPIC_MINT);
  const [position] = derivePositionPda(market, owner);
  const [vaultAuthority] = deriveVaultAuthorityPda(market);

  const userCollateralAccount = getAssociatedTokenAddressSync(
    DEMO_ANTHROPIC_MINT,
    owner,
    false,
    TOKEN_2022_PROGRAM_ID,
    ASSOCIATED_TOKEN_PROGRAM_ID
  );

  const vaultCollateralAccount = getAssociatedTokenAddressSync(
    DEMO_ANTHROPIC_MINT,
    vaultAuthority,
    true,
    TOKEN_2022_PROGRAM_ID,
    ASSOCIATED_TOKEN_PROGRAM_ID
  );

  return program.methods
    .depositCollateral(new BN(amountRaw.toString()))
    .accounts({
      user: owner,
      market,
      position,
      collateralMint: DEMO_ANTHROPIC_MINT,
      userCollateralAccount,
      vaultAuthority,
      vaultCollateralAccount,
      tokenProgram: TOKEN_2022_PROGRAM_ID,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
    })
    .rpc();
}

export async function borrowDebt(
  connection: Connection,
  wallet: AnchorWallet,
  amountRaw: bigint
) {
  const program = getOspreyProgram(connection, wallet);

  const owner = wallet.publicKey;

  const [market] = deriveMarketPda(DEMO_ANTHROPIC_MINT);
  const [position] = derivePositionPda(market, owner);
  const [riskSnapshot] = deriveRiskSnapshotPda(market);
  const [vaultAuthority] = deriveVaultAuthorityPda(market);

  const userDebtAccount = await ensureAssociatedTokenAccount(
    connection,
    wallet,
    DEMO_USDC_MINT
  );

  const debtVault = getAssociatedTokenAddressSync(
    DEMO_USDC_MINT,
    vaultAuthority,
    true,
    TOKEN_2022_PROGRAM_ID,
    ASSOCIATED_TOKEN_PROGRAM_ID
  );

  return program.methods
    .borrow(new BN(amountRaw.toString()))
    .accounts({
      user: owner,
      market,
      position,
      riskSnapshot,
      collateralMint: DEMO_ANTHROPIC_MINT,
      debtMint: DEMO_USDC_MINT,
      userDebtAccount,
      vaultAuthority,
      debtVault,
      tokenProgram: TOKEN_2022_PROGRAM_ID,
    })
    .rpc();
}

export async function repayDebt(
  connection: Connection,
  wallet: AnchorWallet,
  amountRaw: bigint
) {
  const program = getOspreyProgram(connection, wallet);

  const owner = wallet.publicKey;

  const [market] = deriveMarketPda(DEMO_ANTHROPIC_MINT);
  const [position] = derivePositionPda(market, owner);
  const [vaultAuthority] = deriveVaultAuthorityPda(market);

  const userDebtAccount = getAssociatedTokenAddressSync(
    DEMO_USDC_MINT,
    owner,
    false,
    TOKEN_2022_PROGRAM_ID,
    ASSOCIATED_TOKEN_PROGRAM_ID
  );

  const userDebtAccountInfo = await connection.getAccountInfo(userDebtAccount);

  if (!userDebtAccountInfo) {
    throw new Error("You do not have a Demo USDC token account to repay from.");
  }

  const debtVault = getAssociatedTokenAddressSync(
    DEMO_USDC_MINT,
    vaultAuthority,
    true,
    TOKEN_2022_PROGRAM_ID,
    ASSOCIATED_TOKEN_PROGRAM_ID
  );

  return program.methods
    .repay(new BN(amountRaw.toString()))
    .accounts({
      user: owner,
      market,
      position,
      debtMint: DEMO_USDC_MINT,
      userDebtAccount,
      vaultAuthority,
      debtVault,
      tokenProgram: TOKEN_2022_PROGRAM_ID,
    })
    .rpc();
}

export async function withdrawCollateral(
  connection: Connection,
  wallet: AnchorWallet,
  amountRaw: bigint
) {
  const program = getOspreyProgram(connection, wallet);

  const owner = wallet.publicKey;

  const [market] = deriveMarketPda(DEMO_ANTHROPIC_MINT);
  const [position] = derivePositionPda(market, owner);
  const [riskSnapshot] = deriveRiskSnapshotPda(market);
  const [vaultAuthority] = deriveVaultAuthorityPda(market);

  const userCollateralAccount = await ensureAssociatedTokenAccount(
    connection,
    wallet,
    DEMO_ANTHROPIC_MINT
  );

  const vaultCollateralAccount = getAssociatedTokenAddressSync(
    DEMO_ANTHROPIC_MINT,
    vaultAuthority,
    true,
    TOKEN_2022_PROGRAM_ID,
    ASSOCIATED_TOKEN_PROGRAM_ID
  );

  return program.methods
    .withdrawCollateral(new BN(amountRaw.toString()))
    .accounts({
      user: owner,
      market,
      position,
      riskSnapshot,
      collateralMint: DEMO_ANTHROPIC_MINT,
      userCollateralAccount,
      vaultAuthority,
      vaultCollateralAccount,
      tokenProgram: TOKEN_2022_PROGRAM_ID,
    })
    .rpc();
}
