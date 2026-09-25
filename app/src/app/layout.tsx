import type { Metadata } from "next";
import "./globals.css";

import { SolanaProvider } from "@/components/wallet/solana-provider";

export const metadata: Metadata = {
  title: "Osprey Protocol",
  description: "Liquidity-aware lending for tokenized equities on Solana.",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <body>
        <SolanaProvider>{children}</SolanaProvider>
      </body>
    </html>
  );
}
