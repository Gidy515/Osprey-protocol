"use client";

import dynamic from "next/dynamic";

const MarketApp = dynamic(
  () => import("@/components/market/market-app"),
  {
    ssr: false,
    loading: () => (
      <main
        style={{
          maxWidth: 1000,
          margin: "0 auto",
          padding: "40px 24px",
          fontFamily: "Arial, sans-serif",
        }}
      >
        <strong>OSPREY</strong>
        <p>Loading protocol...</p>
      </main>
    ),
  }
);

export default function Home() {
  return <MarketApp />;
}
