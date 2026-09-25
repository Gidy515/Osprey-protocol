"use client";

import { useConnection } from "@solana/wallet-adapter-react";
import Link from "next/link";
import { useEffect, useMemo, useState } from "react";

import { OspreyLogo } from "@/components/brand/osprey-logo";
import { useOspreyMarket } from "@/lib/osprey/use-market";

import styles from "./risk-engine.module.css";

const MAX_SNAPSHOT_AGE_SLOTS = BigInt(300);

function percent(bps: number) {
  return `${(bps / 100).toFixed(2)}%`;
}

function usdc(raw: bigint) {
  return (Number(raw) / 1_000_000).toLocaleString(undefined, {
    minimumFractionDigits: 0,
    maximumFractionDigits: 2,
  });
}

function anthropic(raw: bigint) {
  return (Number(raw) / 1_000_000_000).toLocaleString(undefined, {
    minimumFractionDigits: 0,
    maximumFractionDigits: 4,
  });
}

function shortAddress(value: { toBase58(): string }) {
  const address = value.toBase58();
  return `${address.slice(0, 5)}…${address.slice(-5)}`;
}

export default function RiskEngine() {
  const { connection } = useConnection();

  const {
    market,
    snapshot,
    risk,
    loading,
    error,
  } = useOspreyMarket();

  const [currentSlot, setCurrentSlot] = useState<bigint | null>(null);

  useEffect(() => {
    let cancelled = false;

    async function updateSlot() {
      try {
        const slot = await connection.getSlot("confirmed");

        if (!cancelled) {
          setCurrentSlot(BigInt(slot));
        }
      } catch (error) {
        console.warn("Could not read current Solana slot:", error);
      }
    }

    void updateSlot();

    const interval = window.setInterval(() => {
      void updateSlot();
    }, 10_000);

    return () => {
      cancelled = true;
      window.clearInterval(interval);
    };
  }, [connection]);

  const snapshotAge = useMemo(() => {
    if (!snapshot || currentSlot === null) {
      return null;
    }

    if (currentSlot < snapshot.observedSlot) {
      return null;
    }

    return currentSlot - snapshot.observedSlot;
  }, [currentSlot, snapshot]);

  const snapshotFresh =
    snapshotAge !== null &&
    snapshotAge <= MAX_SNAPSHOT_AGE_SLOTS;

  const recoveryWidth = risk
    ? `${Math.min(100, risk.recoveryBps / 100)}%`
    : "0%";

  const executionWidth = market && risk
    ? `${Math.min(
        100,
        (risk.executionLtvBps / market.maxLtvBps) * 100
      )}%`
    : "0%";

  return (
    <main className={styles.page}>
      <aside className={styles.sidebar}>
        <Link href="/" className={styles.sidebarBrand}>
          <OspreyLogo size={38} />
        </Link>

        <nav className={styles.sidebarNav}>
          <Link href="/app">
            <span>⌂</span>
            Overview
          </Link>

          <Link href="/app#market">
            <span>◫</span>
            Markets
          </Link>

          <Link href="/app#position">
            <span>◇</span>
            Positions
          </Link>

          <Link href="/risk" className={styles.navActive}>
            <span>⌁</span>
            Risk Engine
          </Link>
        </nav>

        <div className={styles.sidebarBottom}>
          <div className={styles.networkStatus}>
            <span />
            Solana Devnet
          </div>

          <p>
            Executable liquidity becomes credit risk data.
          </p>
        </div>
      </aside>

      <div className={styles.main}>
        <header className={styles.topbar}>
          <div className={styles.mobileBrand}>
            <OspreyLogo size={34} />
          </div>

          <p>OSPREY RISK INFRASTRUCTURE</p>

          <Link href="/app" className={styles.launchLink}>
            Open Market →
          </Link>
        </header>

        <div className={styles.content}>
          <section className={styles.hero}>
            <div>
              <p className={styles.eyebrow}>RISK ENGINE</p>

              <h1>
                Executable liquidity,
                <br />
                translated into credit.
              </h1>

              <p className={styles.heroCopy}>
                Osprey measures how much value a reference collateral
                position can actually recover through market liquidity,
                then constrains borrowing power with execution and issuer
                risk.
              </p>
            </div>

            {!loading && market && snapshot && risk && (
              <div
                className={`${styles.liveState} ${
                  snapshotFresh
                    ? styles.liveStateFresh
                    : styles.liveStateStale
                }`}
              >
                <span />

                <div>
                  <strong>
                    {snapshotFresh
                      ? "RISK SNAPSHOT LIVE"
                      : "SNAPSHOT UPDATING"}
                  </strong>

                  <small>
                    {snapshotAge === null
                      ? "Checking freshness"
                      : `${snapshotAge.toString()} slots old`}
                  </small>
                </div>
              </div>
            )}
          </section>

          {loading && (
            <div className={styles.loading}>
              Reading Osprey market and liquidity snapshot…
            </div>
          )}

          {error && (
            <div className={styles.error}>
              <strong>Risk data unavailable.</strong>
              <span>{error}</span>
            </div>
          )}

          {market && snapshot && risk && (
            <>
              <section className={styles.marketStrip}>
                <div className={styles.marketIdentity}>
                  <div className={styles.assetMark}>AN</div>

                  <div>
                    <span>ANTHROPIC MARKET</span>
                    <strong>ANTH / USDC</strong>
                    <small>Tokenized equity · Meteora DLMM</small>
                  </div>
                </div>

                <div className={styles.stripMetric}>
                  <span>PRICE</span>
                  <strong>
                    ${usdc(market.collateralPriceUsdc)}
                  </strong>
                </div>

                <div className={styles.stripMetric}>
                  <span>RECOVERY</span>
                  <strong>{percent(risk.recoveryBps)}</strong>
                </div>

                <div className={styles.stripMetric}>
                  <span>EFFECTIVE LTV</span>
                  <strong className={styles.copper}>
                    {percent(risk.effectiveLtvBps)}
                  </strong>
                </div>
              </section>

              <section className={styles.engineGrid}>
                <article className={styles.liquidityPanel}>
                  <div className={styles.panelHeading}>
                    <div>
                      <p className={styles.eyebrow}>
                        LIQUIDITY SIMULATION
                      </p>

                      <h2>Executable recovery</h2>
                    </div>

                    <span>Meteora DLMM</span>
                  </div>

                  <div className={styles.liquidationFlow}>
                    <div className={styles.flowBlock}>
                      <span>REFERENCE COLLATERAL</span>

                      <strong>
                        {anthropic(snapshot.quoteCollateralIn)} ANTH
                      </strong>

                      <small>
                        $
                        {usdc(
                          market.referenceLiquidationSizeUsdc
                        )}{" "}
                        reference position
                      </small>
                    </div>

                    <div className={styles.arrow}>→</div>

                    <div className={styles.flowBlock}>
                      <span>EXECUTABLE OUTPUT</span>

                      <strong>
                        ${usdc(snapshot.quoteUsdcOut)}
                      </strong>

                      <small>USDC recoverable through liquidity</small>
                    </div>
                  </div>

                  <div className={styles.recovery}>
                    <div className={styles.recoveryHeader}>
                      <span>EXECUTABLE RECOVERY</span>

                      <strong>{percent(risk.recoveryBps)}</strong>
                    </div>

                    <div className={styles.track}>
                      <div style={{ width: recoveryWidth }} />
                    </div>

                    <div className={styles.scale}>
                      <span>$0</span>

                      <span>
                        ${usdc(market.referenceLiquidationSizeUsdc)}
                      </span>
                    </div>
                  </div>

                  <div className={styles.explanation}>
                    <span>01</span>

                    <p>
                      Instead of assuming collateral can be liquidated
                      at its quoted market value, Osprey uses the
                      executable USDC output of a reference liquidation
                      to measure recovery.
                    </p>
                  </div>
                </article>

                <article className={styles.effectivePanel}>
                  <p className={styles.eyebrow}>
                    FINAL CREDIT PARAMETER
                  </p>

                  <span className={styles.effectiveLabel}>
                    EFFECTIVE LTV
                  </span>

                  <strong className={styles.effectiveValue}>
                    {percent(risk.effectiveLtvBps)}
                  </strong>

                  <p>
                    The borrowing limit enforced by Osprey after
                    executable liquidity and issuer risk constraints.
                  </p>

                  <div className={styles.effectiveTrack}>
                    <div
                      style={{
                        width: `${Math.min(
                          100,
                          risk.effectiveLtvBps / 100
                        )}%`,
                      }}
                    />
                  </div>

                  <div className={styles.effectiveScale}>
                    <span>0%</span>
                    <span>{percent(market.maxLtvBps)} market max</span>
                    <span>100%</span>
                  </div>
                </article>
              </section>

              <section className={styles.riskSection}>
                <div className={styles.sectionHeading}>
                  <div>
                    <p className={styles.eyebrow}>
                      RISK PARAMETERS
                    </p>

                    <h2>How borrowing power contracts.</h2>
                  </div>

                  <p>
                    Execution-derived LTV is bounded by the market
                    configuration and then capped by issuer risk.
                  </p>
                </div>

                <div className={styles.riskCards}>
                  <article>
                    <span>01 · MARKET MAXIMUM</span>

                    <strong>{percent(market.maxLtvBps)}</strong>

                    <small>
                      Maximum LTV before liquidity and issuer
                      constraints.
                    </small>
                  </article>

                  <article>
                    <span>02 · EXECUTION LTV</span>

                    <strong>{percent(risk.executionLtvBps)}</strong>

                    <div className={styles.miniTrack}>
                      <div style={{ width: executionWidth }} />
                    </div>

                    <small>
                      Market maximum adjusted by executable recovery.
                    </small>
                  </article>

                  <article>
                    <span>03 · ISSUER CEILING</span>

                    <strong>
                      {percent(market.issuerRiskCeilingBps)}
                    </strong>

                    <small>
                      Independent ceiling for asset administration and
                      issuer risk.
                    </small>
                  </article>

                  <article className={styles.resultCard}>
                    <span>04 · EFFECTIVE LTV</span>

                    <strong>{percent(risk.effectiveLtvBps)}</strong>

                    <small>
                      Minimum of execution-derived LTV and issuer
                      ceiling.
                    </small>
                  </article>
                </div>

                <div className={styles.formula}>
                  <div>
                    <span>Market max</span>
                    <strong>{percent(market.maxLtvBps)}</strong>
                  </div>

                  <b>×</b>

                  <div>
                    <span>Recovery</span>
                    <strong>{percent(risk.recoveryBps)}</strong>
                  </div>

                  <b>→</b>

                  <div>
                    <span>Execution LTV</span>
                    <strong>{percent(risk.executionLtvBps)}</strong>
                  </div>

                  <b>min</b>

                  <div>
                    <span>Issuer ceiling</span>
                    <strong>
                      {percent(market.issuerRiskCeilingBps)}
                    </strong>
                  </div>

                  <b>=</b>

                  <div className={styles.formulaResult}>
                    <span>Effective LTV</span>
                    <strong>{percent(risk.effectiveLtvBps)}</strong>
                  </div>
                </div>
              </section>

              <section className={styles.lowerGrid}>
                <article className={styles.snapshotPanel}>
                  <div className={styles.panelHeading}>
                    <div>
                      <p className={styles.eyebrow}>
                        ONCHAIN SNAPSHOT
                      </p>

                      <h2>Liquidity state</h2>
                    </div>

                    <div
                      className={`${styles.snapshotBadge} ${
                        snapshotFresh
                          ? styles.snapshotFresh
                          : styles.snapshotStale
                      }`}
                    >
                      <span />
                      {snapshotFresh ? "FRESH" : "STALE"}
                    </div>
                  </div>

                  <dl className={styles.dataList}>
                    <div>
                      <dt>Collateral input</dt>
                      <dd>
                        {anthropic(snapshot.quoteCollateralIn)} ANTH
                      </dd>
                    </div>

                    <div>
                      <dt>USDC output</dt>
                      <dd>${usdc(snapshot.quoteUsdcOut)}</dd>
                    </div>

                    <div>
                      <dt>Observed slot</dt>
                      <dd>{snapshot.observedSlot.toString()}</dd>
                    </div>

                    <div>
                      <dt>Current slot</dt>
                      <dd>
                        {currentSlot?.toString() ?? "Reading…"}
                      </dd>
                    </div>

                    <div>
                      <dt>Snapshot age</dt>
                      <dd>
                        {snapshotAge === null
                          ? "Checking…"
                          : `${snapshotAge.toString()} slots`}
                      </dd>
                    </div>

                    <div>
                      <dt>Maximum age</dt>
                      <dd>
                        {MAX_SNAPSHOT_AGE_SLOTS.toString()} slots
                      </dd>
                    </div>
                  </dl>
                </article>

                <article className={styles.snapshotPanel}>
                  <div className={styles.panelHeading}>
                    <div>
                      <p className={styles.eyebrow}>
                        LIQUIDATION PARAMETERS
                      </p>

                      <h2>Market safeguards</h2>
                    </div>
                  </div>

                  <dl className={styles.dataList}>
                    <div>
                      <dt>Liquidation threshold</dt>
                      <dd>
                        {percent(
                          market.liquidationThresholdBps
                        )}
                      </dd>
                    </div>

                    <div>
                      <dt>Liquidation bonus</dt>
                      <dd>
                        {percent(market.liquidationBonusBps)}
                      </dd>
                    </div>

                    <div>
                      <dt>Maximum liquidation</dt>
                      <dd>
                        {percent(market.maxLiquidationBps)}
                      </dd>
                    </div>

                    <div>
                      <dt>Token transfer fee</dt>
                      <dd>{percent(market.transferFeeBps)}</dd>
                    </div>

                    <div>
                      <dt>Minimum LTV</dt>
                      <dd>{percent(market.minLtvBps)}</dd>
                    </div>

                    <div>
                      <dt>Reference size</dt>
                      <dd>
                        $
                        {usdc(
                          market.referenceLiquidationSizeUsdc
                        )}
                      </dd>
                    </div>
                  </dl>
                </article>
              </section>

              <section className={styles.accounts}>
                <div>
                  <p className={styles.eyebrow}>
                    ONCHAIN CONFIGURATION
                  </p>

                  <h2>Market accounts</h2>
                </div>

                <div className={styles.accountGrid}>
                  <div>
                    <span>MARKET PDA</span>
                    <strong title={market.address.toBase58()}>
                      {shortAddress(market.address)}
                    </strong>
                  </div>

                  <div>
                    <span>RISK SNAPSHOT</span>
                    <strong title={snapshot.address.toBase58()}>
                      {shortAddress(snapshot.address)}
                    </strong>
                  </div>

                  <div>
                    <span>METEORA POOL</span>
                    <strong title={market.dlmmPool.toBase58()}>
                      {shortAddress(market.dlmmPool)}
                    </strong>
                  </div>

                  <div>
                    <span>MARKET AUTHORITY</span>
                    <strong title={market.authority.toBase58()}>
                      {shortAddress(market.authority)}
                    </strong>
                  </div>
                </div>
              </section>

              <section className={styles.disclosure}>
                <strong>Devnet implementation</strong>

                <p>
                  This market uses a Token-2022 test asset representing
                  ANTH. Liquidity-risk snapshots are derived from Meteora
                  liquidity and updated by the configured market
                  authority. Osprey is not affiliated with or endorsed
                  by Anthropic.
                </p>

                <Link href="/app">Manage position →</Link>
              </section>

              <footer className={styles.footer}>
                <span>
                  Osprey Protocol · Liquidity-aware credit
                  infrastructure.
                </span>

                <span>
                  ANTH / USDC · Solana Devnet
                </span>
              </footer>
            </>
          )}
        </div>
      </div>
    </main>
  );
}
