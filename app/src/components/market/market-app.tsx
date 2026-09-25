"use client";

import {
  useAnchorWallet,
  useConnection,
  useWallet,
} from "@solana/wallet-adapter-react";
import dynamic from "next/dynamic";
import Link from "next/link";
import { useEffect, useMemo, useState } from "react";

import { getTransactionErrorMessage } from "@/lib/osprey/errors";
import {
  borrowDebt,
  depositCollateral,
  repayDebt,
  withdrawCollateral,
} from "@/lib/osprey/transactions";
import { useOspreyMarket } from "@/lib/osprey/use-market";
import { useOspreyPosition } from "@/lib/osprey/use-position";

import { OspreyLogo } from "@/components/brand/osprey-logo";

import styles from "./market-app.module.css";

const WalletMultiButton = dynamic(
  async () =>
    (await import("@solana/wallet-adapter-react-ui")).WalletMultiButton,
  {
    ssr: false,
  }
);

const MAX_SNAPSHOT_AGE_SLOTS = BigInt(300);

type ActionType = "deposit" | "borrow" | "repay" | "withdraw";

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

function amountToRaw(value: string, decimals: number): bigint | null {
  const trimmed = value.trim();

  if (!/^\d+(\.\d+)?$/.test(trimmed)) {
    return null;
  }

  const [whole, fraction = ""] = trimmed.split(".");

  if (fraction.length > decimals) {
    return null;
  }

  const paddedFraction = fraction.padEnd(decimals, "0");

  try {
    return (
      BigInt(whole) * BigInt(10) ** BigInt(decimals) + BigInt(paddedFraction || "0")
    );
  } catch {
    return null;
  }
}

function actionLabel(action: ActionType) {
  switch (action) {
    case "deposit":
      return "Deposit";
    case "borrow":
      return "Borrow";
    case "repay":
      return "Repay";
    case "withdraw":
      return "Withdraw";
  }
}

export default function MarketApp() {
  const { connection } = useConnection();
  const { connected } = useWallet();
  const anchorWallet = useAnchorWallet();

  const {
    market,
    snapshot,
    risk,
    loading: marketLoading,
    error: marketError,
    refresh: refreshMarket,
  } = useOspreyMarket();

  const {
    position,
    loading: positionLoading,
    error: positionError,
    refresh: refreshPosition,
  } = useOspreyPosition(market?.address ?? null);

  const [selectedAction, setSelectedAction] = useState<ActionType>("deposit");

  const [amount, setAmount] = useState("10");

  const [action, setAction] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [actionSuccess, setActionSuccess] = useState<string | null>(null);

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
    snapshotAge !== null && snapshotAge <= MAX_SNAPSHOT_AGE_SLOTS;

  const collateralValue = useMemo(() => {
    if (!market || !position) {
      return BigInt(0);
    }

    return (
      (position.collateralAmount * market.collateralPriceUsdc) / BigInt(1_000_000_000)
    );
  }, [market, position]);

  const maxDebt = useMemo(() => {
    if (!risk) {
      return BigInt(0);
    }

    return (collateralValue * BigInt(risk.effectiveLtvBps)) / BigInt(10_000);
  }, [collateralValue, risk]);

  const debt = position?.debtAmount ?? BigInt(0);

  const collateral = position?.collateralAmount ?? BigInt(0);

  const availableDebt = useMemo(() => {
    return maxDebt > debt ? maxDebt - debt : BigInt(0);
  }, [maxDebt, debt]);

  const debtUsageBps = useMemo(() => {
    if (maxDebt === BigInt(0)) {
      return 0;
    }

    const value = (debt * BigInt(10_000)) / maxDebt;

    return Number(value > BigInt(10_000) ? BigInt(10_000) : value);
  }, [debt, maxDebt]);

  const recoveryWidth = risk
    ? `${Math.min(100, risk.recoveryBps / 100)}%`
    : "0%";

  const selectedAsset =
    selectedAction === "deposit" || selectedAction === "withdraw"
      ? "ANTH"
      : "USDC";

  const selectedDecimals =
    selectedAction === "deposit" || selectedAction === "withdraw" ? 9 : 6;

  const amountRaw = useMemo(
    () => amountToRaw(amount, selectedDecimals),
    [amount, selectedDecimals]
  );

  const riskSensitive =
    selectedAction === "borrow" || selectedAction === "withdraw";

  const exceedsAvailableBorrow =
    selectedAction === "borrow" &&
    amountRaw !== null &&
    amountRaw > availableDebt;

  const exceedsDebt =
    selectedAction === "repay" && amountRaw !== null && amountRaw > debt;

  const exceedsCollateral =
    selectedAction === "withdraw" &&
    amountRaw !== null &&
    amountRaw > collateral;

  const invalidAmount = amountRaw === null || amountRaw <= BigInt(0);

  const actionDisabled =
    !connected ||
    !!action ||
    invalidAmount ||
    (riskSensitive && !snapshotFresh) ||
    exceedsAvailableBorrow ||
    exceedsDebt ||
    exceedsCollateral;

  function chooseAction(nextAction: ActionType) {
    setSelectedAction(nextAction);
    setActionError(null);
    setActionSuccess(null);

    switch (nextAction) {
      case "deposit":
        setAmount("10");
        break;
      case "borrow":
        setAmount("1000");
        break;
      case "repay":
        setAmount("500");
        break;
      case "withdraw":
        setAmount("1");
        break;
    }
  }

  async function executeAction() {
    if (!anchorWallet) {
      setActionError("Connect your wallet before submitting a transaction.");
      return;
    }

    if (amountRaw === null || amountRaw <= BigInt(0)) {
      setActionError("Enter a valid amount greater than zero.");
      return;
    }

    if (riskSensitive && !snapshotFresh) {
      setActionError(
        "Liquidity risk data is stale. Borrow and Withdraw require a fresh on-chain liquidity snapshot."
      );
      return;
    }

    try {
      setAction(`${actionLabel(selectedAction)}ing...`);
      setActionError(null);
      setActionSuccess(null);

      switch (selectedAction) {
        case "deposit":
          await depositCollateral(connection, anchorWallet, amountRaw);
          break;

        case "borrow":
          await borrowDebt(connection, anchorWallet, amountRaw);
          break;

        case "repay":
          await repayDebt(connection, anchorWallet, amountRaw);
          break;

        case "withdraw":
          await withdrawCollateral(connection, anchorWallet, amountRaw);
          break;
      }

      await Promise.all([refreshPosition(), refreshMarket()]);

      setActionSuccess(
        `${actionLabel(selectedAction)} confirmed successfully.`
      );
    } catch (error) {
      console.warn(`${actionLabel(selectedAction)} transaction failed:`, error);

      setActionError(getTransactionErrorMessage(error));
    } finally {
      setAction(null);
    }
  }

  function actionHint() {
    if (!connected) {
      return "Connect your wallet to interact with this market.";
    }

    if (riskSensitive && !snapshotFresh) {
      return "Liquidity risk data is currently stale. Deposit and repay remain available.";
    }

    if (exceedsAvailableBorrow) {
      return `Maximum currently available: ${usdc(availableDebt)} USDC.`;
    }

    if (exceedsDebt) {
      return `Your current debt is ${usdc(debt)} USDC.`;
    }

    if (exceedsCollateral) {
      return `Your deposited collateral is ${anthropic(collateral)} ANTH.`;
    }

    switch (selectedAction) {
      case "deposit":
        return "Deposit ANTH as collateral. Token-2022 transfer fees are accounted for onchain.";

      case "borrow":
        return `Borrow against the current ${
          risk ? percent(risk.effectiveLtvBps) : "—"
        } liquidity-adjusted LTV.`;

      case "repay":
        return "Repayment reduces your outstanding USDC debt and does not require a fresh risk snapshot.";

      case "withdraw":
        return "Withdrawals are checked against the current liquidity-adjusted borrowing limit.";
    }
  }

  return (
    <main className={styles.page}>
      <aside className={styles.sidebar}>
        <Link href="/" className={styles.sidebarBrand}>
          <OspreyLogo size={38} />
        </Link>

        <nav className={styles.sidebarNav}>
          <a href="#overview" className={styles.navActive}>
            <span>⌂</span>
            Overview
          </a>

          <a href="#market">
            <span>◫</span>
            Markets
          </a>

          <a href="#position">
            <span>◇</span>
            Positions
          </a>

          <Link href="/risk">
            <span>⌁</span>
            Risk Engine
          </Link>
        </nav>

        <div className={styles.sidebarBottom}>
          <div className={styles.networkStatus}>
            <span className={styles.networkDot} />
            Solana Devnet
          </div>

          <p>Liquidity-aware credit infrastructure.</p>
        </div>
      </aside>

      <div className={styles.main}>
        <header className={styles.topbar}>
          <div>
            <div className={styles.mobileBrand}>
              <OspreyLogo size={34} />
            </div>

            <p className={styles.topbarEyebrow}>
              TOKENIZED EQUITY CREDIT
            </p>
          </div>

          <WalletMultiButton />
        </header>

        <div className={styles.content}>
          <section id="overview" className={styles.overviewHeader}>
            <div>
              <p className={styles.eyebrow}>OVERVIEW</p>
              <h1>Portfolio Overview</h1>

              <p>
                Manage tokenized equity collateral and stablecoin credit
                against Osprey&apos;s liquidity-aware borrowing limits.
              </p>
            </div>

            <div
              className={`${styles.marketState} ${
                snapshotFresh ? styles.marketStateLive : styles.marketStateStale
              }`}
            >
              <span />
              {snapshotFresh ? "Market live" : "Liquidity stale"}
            </div>
          </section>

          {marketLoading && (
            <div className={styles.loading}>Loading Osprey market…</div>
          )}

          {marketError && (
            <div
              role="alert"
              className={`${styles.notice} ${styles.noticeError}`}
            >
              {marketError}
            </div>
          )}

          {market && snapshot && risk && (
            <>
              <section className={styles.portfolioMetrics}>
                <article>
                  <span>COLLATERAL VALUE</span>
                  <strong>${usdc(collateralValue)}</strong>
                  <small>{anthropic(collateral)} ANTH deposited</small>
                </article>

                <article>
                  <span>BORROWED</span>
                  <strong>${usdc(debt)}</strong>
                  <small>USDC outstanding</small>
                </article>

                <article>
                  <span>BORROW LIMIT</span>
                  <strong>${usdc(maxDebt)}</strong>
                  <small>
                    At {percent(risk.effectiveLtvBps)} effective LTV
                  </small>
                </article>

                <article>
                  <span>UTILIZATION</span>
                  <strong>{percent(debtUsageBps)}</strong>

                  <div className={styles.utilizationTrack}>
                    <div
                      style={{
                        width: `${Math.min(100, debtUsageBps / 100)}%`,
                      }}
                    />
                  </div>
                </article>
              </section>

              <section id="market" className={styles.marketPanel}>
                <div className={styles.marketHeader}>
                  <div className={styles.marketIdentity}>
                    <div
                      className={styles.assetIcon}
                      aria-label="Anthropic market"
                    >
                      <img
                        src="/brand/anthropic-logo.svg"
                        alt=""
                        aria-hidden="true"
                      />
                    </div>

                    <div>
                      <div className={styles.marketNameRow}>
                        <h2>Anthropic</h2>

                        <span
                          className={
                            snapshotFresh
                              ? styles.liveBadge
                              : styles.staleBadge
                          }
                        >
                          ● {snapshotFresh ? "LIVE" : "STALE"}
                        </span>
                      </div>

                      <p>
                        ANTH / USDC · Tokenized equity market · Meteora DLMM
                      </p>
                    </div>
                  </div>

                  <div className={styles.marketPrice}>
                    <span>MARKET PRICE</span>
                    <strong>
                      ${usdc(market.collateralPriceUsdc)}
                    </strong>
                    <small>per ANTH</small>
                  </div>
                </div>

                <div className={styles.marketDataGrid}>
                  <article>
                    <span>EXECUTABLE RECOVERY</span>
                    <strong>{percent(risk.recoveryBps)}</strong>

                    <div className={styles.recoveryTrack}>
                      <div style={{ width: recoveryWidth }} />
                    </div>

                    <small>
                      ${usdc(snapshot.quoteUsdcOut)} executable from $
                      {usdc(market.referenceLiquidationSizeUsdc)}
                    </small>
                  </article>

                  <article className={styles.effectiveMetric}>
                    <span>EFFECTIVE LTV</span>
                    <strong>{percent(risk.effectiveLtvBps)}</strong>
                    <small>Current borrowing power</small>
                  </article>

                  <article>
                    <span>MARKET MAXIMUM</span>
                    <strong>{percent(market.maxLtvBps)}</strong>
                    <small>Configured upper bound</small>
                  </article>

                  <article>
                    <span>ISSUER CEILING</span>
                    <strong>
                      {percent(market.issuerRiskCeilingBps)}
                    </strong>
                    <small>Asset-level risk ceiling</small>
                  </article>
                </div>

                <div className={styles.riskEquation}>
                  <div>
                    <span>MARKET MAX</span>
                    <strong>{percent(market.maxLtvBps)}</strong>
                  </div>

                  <b>×</b>

                  <div>
                    <span>RECOVERY</span>
                    <strong>{percent(risk.recoveryBps)}</strong>
                  </div>

                  <b>→</b>

                  <div>
                    <span>EXECUTION LTV</span>
                    <strong>{percent(risk.executionLtvBps)}</strong>
                  </div>

                  <b>∩</b>

                  <div>
                    <span>ISSUER CEILING</span>
                    <strong>
                      {percent(market.issuerRiskCeilingBps)}
                    </strong>
                  </div>

                  <b>→</b>

                  <div className={styles.equationResult}>
                    <span>EFFECTIVE</span>
                    <strong>{percent(risk.effectiveLtvBps)}</strong>
                  </div>
                </div>
              </section>

              <div id="position" className={styles.positionGrid}>
                <section className={styles.positionPanel}>
                  <div className={styles.panelHeader}>
                    <div>
                      <p className={styles.cardLabel}>YOUR POSITION</p>
                      <h2>ANTH collateral</h2>
                    </div>

                    {connected && (
                      <span
                        className={`${styles.healthBadge} ${
                          debtUsageBps >= 9000
                            ? styles.healthDanger
                            : debtUsageBps >= 7500
                              ? styles.healthWarning
                              : ""
                        }`}
                      >
                        {debtUsageBps >= 9000
                          ? "High utilization"
                          : debtUsageBps >= 7500
                            ? "Watch position"
                            : "Healthy"}
                      </span>
                    )}
                  </div>

                  {!connected && (
                    <div className={styles.emptyState}>
                      <strong>No wallet connected</strong>
                      <p>
                        Connect your wallet to view your Osprey lending
                        position.
                      </p>
                    </div>
                  )}

                  {connected && positionLoading && (
                    <div className={styles.loading}>Loading position…</div>
                  )}

                  {positionError && (
                    <div
                      role="alert"
                      className={`${styles.notice} ${styles.noticeError}`}
                    >
                      {positionError}
                    </div>
                  )}

                  {connected && !positionLoading && (
                    <div className={styles.positionStats}>
                      <div>
                        <span>COLLATERAL</span>
                        <strong>{anthropic(collateral)} ANTH</strong>
                        <small>${usdc(collateralValue)}</small>
                      </div>

                      <div>
                        <span>DEBT</span>
                        <strong>${usdc(debt)}</strong>
                        <small>USDC</small>
                      </div>
                    </div>
                  )}
                </section>

                <section className={styles.borrowPanel}>
                  <div className={styles.panelHeader}>
                    <div>
                      <p className={styles.cardLabel}>BORROWING POWER</p>
                      <h2>${usdc(availableDebt)} available</h2>
                    </div>

                    <span className={styles.ltvValue}>
                      {percent(risk.effectiveLtvBps)} LTV
                    </span>
                  </div>

                  <div className={styles.borrowTrack}>
                    <div
                      style={{
                        width: `${Math.min(100, debtUsageBps / 100)}%`,
                      }}
                    />
                  </div>

                  <div className={styles.borrowScale}>
                    <span>$0</span>
                    <span>${usdc(maxDebt)} limit</span>
                  </div>

                  <div className={styles.snapshotStatus}>
                    <div>
                      <span
                        className={`${styles.statusDot} ${
                          snapshotFresh
                            ? styles.statusFresh
                            : styles.statusStale
                        }`}
                      />

                      {snapshotFresh
                        ? "Liquidity snapshot fresh"
                        : "Liquidity snapshot stale"}
                    </div>

                    <span>
                      {snapshotAge === null
                        ? "Checking age"
                        : `${snapshotAge.toString()} slots old`}
                    </span>
                  </div>
                </section>
              </div>

              <section className={styles.managementPanel}>
                <div className={styles.managementHeader}>
                  <div>
                    <p className={styles.cardLabel}>
                      POSITION MANAGEMENT
                    </p>

                    <h2>Manage collateral and debt</h2>
                  </div>

                  <span>
                    Risk-increasing actions require fresh liquidity data.
                  </span>
                </div>

                <div className={styles.actionTabs}>
                  {(
                    [
                      "deposit",
                      "borrow",
                      "repay",
                      "withdraw",
                    ] as ActionType[]
                  ).map((item) => (
                    <button
                      key={item}
                      type="button"
                      className={`${styles.actionTab} ${
                        selectedAction === item
                          ? styles.actionTabActive
                          : ""
                      }`}
                      onClick={() => chooseAction(item)}
                      disabled={!!action}
                    >
                      {actionLabel(item)}
                    </button>
                  ))}
                </div>

                <div className={styles.actionBody}>
                  <div className={styles.actionCopy}>
                    <span>
                      {actionLabel(selectedAction).toUpperCase()}{" "}
                      {selectedAsset}
                    </span>

                    <h3>
                      {selectedAction === "deposit" &&
                        "Supply tokenized equity collateral."}

                      {selectedAction === "borrow" &&
                        "Borrow USDC against available credit."}

                      {selectedAction === "repay" &&
                        "Reduce your outstanding USDC debt."}

                      {selectedAction === "withdraw" &&
                        "Remove collateral within your borrowing limit."}
                    </h3>

                    <p>{actionHint()}</p>
                  </div>

                  <div className={styles.actionControls}>
                    <div className={styles.amountField}>
                      <input
                        className={styles.amountInput}
                        type="text"
                        inputMode="decimal"
                        value={amount}
                        onChange={(event) => {
                          setAmount(event.target.value);
                          setActionError(null);
                          setActionSuccess(null);
                        }}
                        placeholder="0.00"
                        aria-label={`${actionLabel(selectedAction)} amount`}
                      />

                      <span className={styles.amountAsset}>
                        {selectedAsset}
                      </span>
                    </div>

                    <button
                      type="button"
                      className={styles.primaryButton}
                      onClick={executeAction}
                      disabled={actionDisabled}
                    >
                      {action ?? actionLabel(selectedAction)}
                    </button>
                  </div>
                </div>

                {!connected && (
                  <div className={styles.staleNotice}>
                    Connect your wallet before managing a position.
                  </div>
                )}

                {!snapshotFresh && connected && (
                  <div className={styles.staleNotice}>
                    Liquidity risk data is stale. Borrow and Withdraw require a fresh
                    on-chain liquidity snapshot. Deposit and Repay remain
                    available because they do not increase position risk.
                  </div>
                )}

                {action && (
                  <div
                    role="status"
                    className={`${styles.notice} ${styles.noticeLoading}`}
                  >
                    Confirm the transaction in your wallet.
                  </div>
                )}

                {actionSuccess && (
                  <div
                    role="status"
                    className={`${styles.notice} ${styles.noticeSuccess}`}
                  >
                    {actionSuccess}
                  </div>
                )}

                {actionError && (
                  <div
                    role="alert"
                    className={`${styles.notice} ${styles.noticeError}`}
                  >
                    <strong>Transaction not completed.</strong>{" "}
                    {actionError}
                  </div>
                )}
              </section>

              <section className={styles.disclosure}>
                <div>
                  <strong>Devnet market</strong>

                  <p>
                    This market uses a Token-2022 test asset representing ANTH
                    for Osprey&apos;s deployed devnet credit market. Osprey is
                    not affiliated with or endorsed by Anthropic.
                  </p>
                </div>

                <Link href="/risk">
                  Inspect Risk Engine →
                </Link>
              </section>

              <footer className={styles.footer}>
                <span>
                  Osprey Protocol · Liquidity-aware credit infrastructure for
                  tokenized equities.
                </span>

                <span>Solana Devnet · ANTH / USDC</span>
              </footer>
            </>
          )}
        </div>
      </div>
    </main>
  );
}
