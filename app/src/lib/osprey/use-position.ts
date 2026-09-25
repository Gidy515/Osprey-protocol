"use client";

import {
  useConnection,
  useWallet,
} from "@solana/wallet-adapter-react";
import { PublicKey } from "@solana/web3.js";
import {
  useCallback,
  useEffect,
  useState,
} from "react";

import {
  fetchPosition,
  OspreyPosition,
} from "@/lib/osprey/position";

export function useOspreyPosition(
  market: PublicKey | null
) {
  const { connection } = useConnection();
  const { publicKey } = useWallet();

  const [position, setPosition] =
    useState<OspreyPosition | null>(null);

  const [loading, setLoading] =
    useState(false);

  const [error, setError] =
    useState<string | null>(null);

  const refresh = useCallback(async () => {
    if (!market || !publicKey) {
      setPosition(null);
      setLoading(false);
      return;
    }

    try {
      setLoading(true);
      setError(null);

      const result = await fetchPosition(
        connection,
        market,
        publicKey
      );

      setPosition(result);
    } catch (err) {
      setError(
        err instanceof Error
          ? err.message
          : "Failed to load position."
      );
    } finally {
      setLoading(false);
    }
  }, [connection, market, publicKey]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  return {
    position,
    loading,
    error,
    refresh,
  };
}
