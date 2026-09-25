"use client";

import dynamic from "next/dynamic";

const RiskEngine = dynamic(
  () => import("@/components/risk/risk-engine"),
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
        <p style={{ color: "#999b94" }}>Loading risk engine...</p>
      </main>
    ),
  }
);

export default function RiskPage() {
  return <RiskEngine />;
}
