"use client";

import { useConnection } from "@solana/wallet-adapter-react";
import { useCallback, useEffect, useState } from "react";

import {
  fetchDemoMarket,
  LiquidityRiskSnapshot,
  OspreyMarket,
  RiskMetrics,
} from "@/lib/osprey/market";

const MARKET_REFRESH_INTERVAL_MS = 5_000;

export function useOspreyMarket() {
  const { connection } = useConnection();

  const [market, setMarket] = useState<OspreyMarket | null>(null);

  const [snapshot, setSnapshot] = useState<LiquidityRiskSnapshot | null>(null);

  const [risk, setRisk] = useState<RiskMetrics | null>(null);

  const [loading, setLoading] = useState(true);

  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(
    async (showLoading = false) => {
      try {
        if (showLoading) {
          setLoading(true);
        }

        setError(null);

        const result = await fetchDemoMarket(connection);

        setMarket(result.market);
        setSnapshot(result.snapshot);
        setRisk(result.risk);
      } catch (err) {
        setError(err instanceof Error ? err.message : "Failed to load market.");
      } finally {
        if (showLoading) {
          setLoading(false);
        }
      }
    },
    [connection]
  );

  useEffect(() => {
    let cancelled = false;

    async function initialLoad() {
      try {
        setLoading(true);
        setError(null);

        const result = await fetchDemoMarket(connection);

        if (cancelled) {
          return;
        }

        setMarket(result.market);
        setSnapshot(result.snapshot);
        setRisk(result.risk);
      } catch (err) {
        if (cancelled) {
          return;
        }

        setError(err instanceof Error ? err.message : "Failed to load market.");
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    }

    void initialLoad();

    const interval = window.setInterval(() => {
      if (!cancelled) {
        void refresh(false);
      }
    }, MARKET_REFRESH_INTERVAL_MS);

    return () => {
      cancelled = true;
      window.clearInterval(interval);
    };
  }, [connection, refresh]);

  return {
    market,
    snapshot,
    risk,
    loading,
    error,
    refresh: () => refresh(false),
  };
}
