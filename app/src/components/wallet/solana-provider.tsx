"use client";

import {
  ConnectionProvider,
  WalletProvider,
} from "@solana/wallet-adapter-react";
import {
  WalletModalProvider,
} from "@solana/wallet-adapter-react-ui";
import { useMemo } from "react";

import { SOLANA_RPC_URL } from "@/lib/solana/constants";

import "@solana/wallet-adapter-react-ui/styles.css";

export function SolanaProvider({
  children,
}: {
  children: React.ReactNode;
}) {
  const wallets = useMemo(() => [], []);

  return (
    <ConnectionProvider endpoint={SOLANA_RPC_URL}>
      <WalletProvider
        wallets={wallets}
        autoConnect
      >
        <WalletModalProvider>
          {children}
        </WalletModalProvider>
      </WalletProvider>
    </ConnectionProvider>
  );
}
