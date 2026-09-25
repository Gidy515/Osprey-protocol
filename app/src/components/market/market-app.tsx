"use client";

import {
  useAnchorWallet,
  useConnection,
  useWallet,
} from "@solana/wallet-adapter-react";
import dynamic from "next/dynamic";
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
        "Market liquidity data is being refreshed. Risk-sensitive actions are temporarily unavailable."
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
        return "Deposit Demo ANTH as collateral. Token-2022 transfer fees are accounted for on-chain.";

      case "borrow":
        return `Borrow against the current ${
          risk ? percent(risk.effectiveLtvBps) : "—"
        } liquidity-adjusted LTV.`;

      case "repay":
        return "Repayment reduces your outstanding Demo USDC debt and does not require a fresh risk snapshot.";

      case "withdraw":
        return "Withdrawals are checked against the current liquidity-adjusted borrowing limit.";
    }
  }

  return (
    <main className={styles.page}>
      <div className={styles.shell}>
        <header className={styles.header}>
          <div className={styles.brand}>
            <div className={styles.brandMark}>O</div>

            <div>
              <div className={styles.brandName}>OSPREY</div>
              <div className={styles.network}>Liquidity-aware lending</div>
            </div>
          </div>

          <WalletMultiButton />
        </header>

        <section className={styles.hero}>
          <p className={styles.eyebrow}>
            Lending infrastructure for tokenized equities
          </p>

          <h1 className={styles.heroTitle}>
            Borrow against what markets can actually recover.
          </h1>

          <p className={styles.heroCopy}>
            Osprey adjusts borrowing power using executable liquidation
            liquidity and issuer risk instead of relying on a static collateral
            ratio alone.
          </p>
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
            <div className={styles.marketBar}>
              <div className={styles.marketIdentity}>
                <div className={styles.assetIcon}>A</div>

                <div>
                  <h2 className={styles.marketTitle}>ANTH / USDC</h2>

                  <p className={styles.marketSubtitle}>
                    Demo Anthropic collateral · Meteora DLMM liquidity
                  </p>
                </div>
              </div>

              <span className={styles.devnetBadge}>DEVNET</span>
            </div>

            <div className={styles.topGrid}>
              <section className={styles.card}>
                <p className={styles.cardLabel}>Osprey Risk Engine</p>

                <h2 className={styles.cardHeading}>
                  Liquidity-adjusted borrowing power
                </h2>

                <div className={styles.riskHero}>
                  <div>
                    <div className={styles.effectiveLabel}>Effective LTV</div>

                    <div className={styles.effectiveValue}>
                      {percent(risk.effectiveLtvBps)}
                    </div>
                  </div>

                  <div className={styles.riskSteps}>
                    <div className={styles.riskStep}>
                      <span className={styles.riskStepLabel}>
                        Static maximum
                      </span>

                      <span className={styles.riskStepValue}>
                        {percent(market.maxLtvBps)}
                      </span>
                    </div>

                    <div className={styles.riskStep}>
                      <span className={styles.riskStepLabel}>
                        Execution LTV
                      </span>

                      <span
                        className={`${styles.riskStepValue} ${styles.copper}`}
                      >
                        {percent(risk.executionLtvBps)}
                      </span>
                    </div>

                    <div className={styles.riskStep}>
                      <span className={styles.riskStepLabel}>
                        Issuer ceiling
                      </span>

                      <span className={styles.riskStepValue}>
                        {percent(market.issuerRiskCeilingBps)}
                      </span>
                    </div>
                  </div>
                </div>
              </section>

              <section className={styles.card}>
                <p className={styles.cardLabel}>Executable Liquidity</p>

                <h2 className={styles.cardHeading}>Liquidation recovery</h2>

                <div className={styles.liquidityFlow}>
                  <div className={styles.flowNumbers}>
                    <div>
                      <div className={styles.flowValue}>
                        ${usdc(market.referenceLiquidationSizeUsdc)}
                      </div>

                      <div className={styles.flowCaption}>
                        reference liquidation
                      </div>
                    </div>

                    <div className={styles.flowArrow}>→</div>

                    <div>
                      <div className={styles.flowValue}>
                        ${usdc(snapshot.quoteUsdcOut)}
                      </div>

                      <div className={styles.flowCaption}>
                        executable output
                      </div>
                    </div>
                  </div>

                  <div className={styles.recoveryTrack}>
                    <div
                      className={styles.recoveryFill}
                      style={{
                        width: recoveryWidth,
                      }}
                    />
                  </div>

                  <div className={styles.recoveryMeta}>
                    <span>Recovery</span>
                    <strong>{percent(risk.recoveryBps)}</strong>
                  </div>
                </div>

                <div className={styles.statusRow}>
                  <div className={styles.status}>
                    <span
                      className={`${styles.statusDot} ${
                        snapshotFresh ? styles.statusFresh : styles.statusStale
                      }`}
                    />

                    {snapshotFresh ? "Liquidity live" : "Liquidity updating"}
                  </div>

                  <span className={styles.slot}>
                    {snapshotAge === null
                      ? "Checking snapshot"
                      : `${snapshotAge.toString()} slots old`}
                  </span>
                </div>
              </section>
            </div>

            <section className={`${styles.card} ${styles.positionCard}`}>
              <div className={styles.positionHeader}>
                <div>
                  <p className={styles.cardLabel}>Your Position</p>

                  <h2 className={styles.cardHeading}>
                    ANTH collateral position
                  </h2>
                </div>

                {connected && risk && (
                  <div className={styles.slot}>
                    Debt capacity used {percent(debtUsageBps)}
                  </div>
                )}
              </div>

              {!connected && (
                <div className={styles.loading}>
                  Connect your wallet to open or manage a lending position.
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
                <>
                  <div className={styles.positionMetrics}>
                    <div className={styles.metric}>
                      <div className={styles.metricLabel}>Collateral</div>

                      <div className={styles.metricValue}>
                        {anthropic(collateral)} ANTH
                      </div>
                    </div>

                    <div className={styles.metric}>
                      <div className={styles.metricLabel}>Collateral value</div>

                      <div className={styles.metricValue}>
                        ${usdc(collateralValue)}
                      </div>
                    </div>

                    <div className={styles.metric}>
                      <div className={styles.metricLabel}>Debt</div>

                      <div className={styles.metricValue}>${usdc(debt)}</div>
                    </div>

                    <div className={styles.metric}>
                      <div className={styles.metricLabel}>Available</div>

                      <div className={styles.metricValue}>
                        ${usdc(availableDebt)}
                      </div>
                    </div>
                  </div>

                  <div className={styles.actionArea}>
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

                    <div className={styles.actionForm}>
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

                    <p className={styles.actionHint}>{actionHint()}</p>

                    {!snapshotFresh && (
                      <div className={styles.staleNotice}>
                        Market liquidity is being refreshed. Borrow and Withdraw
                        are temporarily unavailable. Deposit and Repay remain
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
                  </div>
                </>
              )}
            </section>

            <footer className={styles.footer}>
              <span>
                Osprey Protocol · Liquidity-aware lending for tokenized
                equities.
              </span>

              <span>Devnet demonstration · Demo ANTH / Demo USDC</span>
            </footer>
          </>
        )}
      </div>
    </main>
  );
}
