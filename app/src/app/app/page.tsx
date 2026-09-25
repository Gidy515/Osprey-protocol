"use client";

import dynamic from "next/dynamic";

const MarketApp = dynamic(
  () => import("@/components/market/market-app"),
  {
    ssr: false,
    loading: () => (
      <main
        style={{
          minHeight: "100vh",
          background: "#0d0f0d",
          color: "#f4f0e8",
          padding: "40px 24px",
          fontFamily: "Arial, Helvetica, sans-serif",
        }}
      >
        <strong>OSPREY</strong>
        <p style={{ color: "#a7a59e" }}>Loading protocol...</p>
      </main>
    ),
  }
);

export default function AppPage() {
  return <MarketApp />;
}
