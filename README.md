# Osprey Protocol

**Liquidity-aware lending for tokenized equities on Solana.**

Osprey Protocol is a collateralized lending protocol that determines borrowing capacity not only from the nominal value of collateral, but also from the liquidity available to actually liquidate that collateral.

Instead of asking only:

> **“What is this collateral worth?”**

Osprey also asks:

> **“If this position had to be liquidated, how much value could actually be recovered?”**

The MVP is designed around **PreStocks** collateral, **USDC-denominated debt**, **Token-2022**, and executable liquidity from **Meteora DLMM**.

---

## The Problem

Traditional overcollateralized lending protocols generally derive borrowing capacity from a collateral price and a configured Loan-to-Value ratio:

```text
Collateral Value × LTV = Maximum Debt
```

That model works best when collateral has deep, reliable liquidity.

Tokenized equities and other emerging real-world assets introduce another dimension of risk.

A token may have a valid market price while still having limited executable liquidity.

For example, a position may appear to contain $50,000 of collateral, but if liquidating a meaningful portion of that position causes severe price impact or encounters insufficient liquidity, the protocol may recover substantially less than the nominal collateral value.

This creates a distinction between:

```text
Market Value
```

and:

```text
Recoverable Liquidation Value
```

Osprey incorporates this distinction directly into lending risk.

---

## The Idea

Osprey combines two major collateral risk dimensions:

```text
Collateral Risk
│
├── Execution Risk
│   └── How much value can be recovered through available liquidity?
│
└── Issuer Risk
    └── What administrative control exists over the collateral token?
```

Execution risk produces a dynamic LTV based on expected liquidation recovery.

Issuer risk imposes an additional ceiling.

The protocol then uses the more conservative value:

```text
execution-derived LTV
        │
        ▼
min(execution-derived LTV, issuer risk ceiling)
        │
        ▼
effective LTV
```

The resulting **effective LTV** determines borrowing capacity and is also used when validating whether a liquidation restores a position to sufficient health.

---

## How Osprey Works

At a high level:

```text
PreStock Collateral
        │
        ├───────────────┐
        │               │
        ▼               ▼
Token / Issuer      Meteora DLMM
    Risk          Execution Liquidity
        │               │
        │               ▼
        │        Liquidity Risk Snapshot
        │               │
        └───────┬───────┘
                ▼
         Risk Assessment
                │
                ▼
          Effective LTV
                │
        ┌───────┴────────┐
        ▼                ▼
     Borrow          Liquidation
                         │
                         ▼
                  Meteora Swap2 CPI
                         │
                         ▼
                  Actual USDC Output
                         │
                         ▼
                Position Accounting
```

---

# Liquidity-Aware Risk Model

## 1. Reference Liquidation Size

Each market defines a reference liquidation notional.

The current MVP uses:

```text
$5,000
```

represented in USDC base units as:

```rust
5_000_000_000
```

The liquidity updater provides an execution quote representing the expected USDC recovery for the reference liquidation size.

---

## 2. Recovery Ratio

Osprey compares executable USDC output against the reference liquidation value.

Conceptually:

```text
Recovery Ratio =
    Executable USDC Output
    ----------------------
    Reference Liquidation Value
```

The ratio is capped at 100%.

Receiving unusually favorable execution therefore cannot increase the configured maximum LTV.

---

## 3. Execution-Derived LTV

The recovery ratio scales the market's maximum LTV:

```text
Execution LTV =
    Max LTV × Recovery Ratio
```

The resulting value is bounded by:

```text
minimum LTV <= execution LTV <= maximum LTV
```

For the current MVP:

```text
Maximum LTV = 65%
Minimum LTV = 35%
```

---

## 4. Issuer Risk Ceiling

PreStocks introduce risks beyond market liquidity.

Token-2022 assets may expose administrative capabilities such as:

- permanent delegate authority
- freeze or pause authority
- transfer fees
- other issuer-controlled extensions

These risks cannot necessarily be inferred from DEX liquidity.

Osprey therefore applies a separate issuer-risk ceiling.

Current MVP:

```text
Issuer Risk Ceiling = 60%
```

The final LTV is:

```text
Effective LTV =
    min(Execution LTV, Issuer Risk Ceiling)
```

---

## Example

Assume:

```text
Reference liquidation size = $5,000
Maximum LTV               = 65%
Minimum LTV               = 35%
Issuer risk ceiling       = 60%
```

Suppose executable liquidity indicates approximately:

```text
$4,915 recovered from $5,000
```

Recovery is approximately:

```text
98.3%
```

Execution-derived LTV is therefore approximately:

```text
65% × 98.3% ≈ 63.9%
```

Issuer risk then caps this:

```text
min(63.9%, 60%) = 60%
```

So the market operates at an effective LTV of approximately:

```text
60%
```

If executable recovery later deteriorates to:

```text
$4,000 / $5,000 = 80%
```

then:

```text
Execution LTV = 65% × 80%
              = 52%
```

and:

```text
Effective LTV = min(52%, 60%)
              = 52%
```

Borrowing capacity therefore contracts as executable liquidation conditions deteriorate.

---

# Protocol Architecture

The Anchor program is organized around seven primary instructions:

```text
initialize
deposit_collateral
update_liquidity_risk
borrow
repay
withdraw_collateral
liquidate
```

A typical position lifecycle is:

```text
Initialize Market
       │
       ▼
Deposit Collateral
       │
       ▼
Update Liquidity Risk
       │
       ▼
     Borrow
       │
   ┌───┴─────────────┐
   ▼                 ▼
 Repay        Market/Risk Changes
   │                 │
   ▼                 ▼
Withdraw         Liquidation
                     │
                     ▼
               Meteora Swap2
```

---

# Protocol Instructions

## `initialize`

Creates and configures a lending market.

The market stores parameters including:

- collateral mint
- debt mint
- collateral price
- Meteora DLMM pool
- market authority
- maximum LTV
- minimum LTV
- liquidation threshold
- liquidation bonus
- maximum liquidation percentage
- reference liquidation size
- issuer-risk ceiling
- transfer-fee configuration

Each collateral market is independently configured.

---

## `deposit_collateral`

Deposits Token-2022 collateral into the protocol vault.

The protocol measures the vault balance before and after the transfer so that the position is credited with the amount **actually received**.

This is important for Token-2022 collateral using transfer fees.

Conceptually:

```text
User sends 100 tokens
        │
        ▼
Token-2022 transfer fee applies
        │
        ▼
Vault receives actual amount
        │
        ▼
Position credited using vault balance delta
```

This prevents accounting from assuming that the requested transfer amount always equals the amount received by the protocol.

---

## `update_liquidity_risk`

Updates the market's `LiquidityRiskSnapshot`.

The snapshot records:

```text
market
quote_collateral_in
quote_usdc_out
observed_slot
```

The update is restricted to the configured market authority.

The protocol validates snapshot freshness before using it for risk-sensitive operations.

### Current MVP trust model

The current implementation should be described as:

> **Meteora-derived, on-chain enforced, authority-updated liquidity risk snapshots.**

The risk calculation and enforcement happen on-chain, but the current MVP does not independently derive the complete DLMM execution quote from Meteora state inside the lending program.

A production version could replace the trusted updater with a permissionless or independently verifiable liquidity-state mechanism.

---

## `borrow`

Allows a user to borrow debt assets against deposited collateral.

The instruction:

1. determines the collateral's current configured USDC value,
2. loads the latest liquidity-risk snapshot,
3. verifies that the snapshot is fresh,
4. calculates the execution-derived LTV,
5. applies the issuer-risk ceiling,
6. derives the position's maximum debt,
7. ensures the requested borrow remains within that capacity,
8. verifies sufficient protocol debt liquidity,
9. transfers debt tokens to the borrower,
10. updates position debt.

Conceptually:

```text
Collateral
    │
    ▼
Collateral Value
    │
    ├──── Liquidity Snapshot
    │
    ▼
Risk Assessment
    │
    ▼
Effective LTV
    │
    ▼
Maximum Debt
    │
    ▼
Borrow Validation
```

---

## `repay`

Repays outstanding position debt.

Repayment intentionally does not depend on a fresh liquidity-risk snapshot.

A user should still be able to improve the solvency of a position when the liquidity snapshot is stale or temporarily unavailable.

---

## `withdraw_collateral`

Allows collateral to be withdrawn while preserving required position health.

For positions with outstanding debt, Osprey:

1. calculates collateral remaining after the proposed withdrawal,
2. loads the liquidity-risk snapshot,
3. verifies freshness,
4. recalculates the effective LTV,
5. calculates maximum debt supported by the remaining collateral,
6. rejects withdrawals that would exceed that debt capacity.

This prevents users from withdrawing collateral that is required to support existing debt.

---

## `liquidate`

Liquidates unhealthy positions through Meteora DLMM.

The liquidation path performs an actual `Swap2` CPI rather than relying only on an estimated liquidation value.

High-level flow:

```text
Position becomes unhealthy
        │
        ▼
Validate liquidation eligibility
        │
        ▼
Validate liquidation size / cap
        │
        ▼
Load liquidity-risk snapshot
        │
        ▼
Meteora Swap2 CPI
        │
        ▼
Collateral sold
        │
        ▼
USDC enters debt vault
        │
        ▼
Measure actual USDC recovered
        │
        ▼
Validate recovery restores sufficient health
        │
        ▼
Update collateral + debt accounting
```

The lending program records the debt vault balance before and after the Meteora CPI.

Therefore:

```text
Actual USDC Recovered =
    Debt Vault Balance After Swap
    -
    Debt Vault Balance Before Swap
```

Debt reduction is based on actual execution output rather than merely trusting the requested `min_amount_out`.

---

# Liquidation Rules

The MVP uses a static liquidation threshold of:

```text
70%
```

A position becomes eligible for liquidation when its debt exceeds the amount supported at this threshold.

The effective liquidity-aware LTV is then used when evaluating whether the liquidation restores sufficient position health.

The current maximum collateral that may be sold in one liquidation is:

```text
25%
```

This bounds individual liquidation size.

The current configured liquidation bonus is:

```text
5%
```

---

# Onchain State

## `MarketConfig`

Stores market-level configuration.

```rust
MarketConfig {
    collateral_mint
    debt_mint
    collateral_price_usdc
    dlmm_pool
    authority
    max_ltv_bps
    min_ltv_bps
    liquidation_threshold_bps
    liquidation_bonus_bps
    max_liquidation_bps
    reference_liquidation_size_usdc
    issuer_risk_ceiling_bps
    transfer_fee_bps
    bump
}
```

---

## `Position`

Stores a user's position for a specific market.

```rust
Position {
    owner
    market
    collateral_amount
    debt_amount
    bump
}
```

Markets are independent.

The current MVP does not implement portfolio margin or cross-collateral borrowing.

---

## `LiquidityRiskSnapshot`

Stores the latest execution-liquidity observation used by the risk engine.

```rust
LiquidityRiskSnapshot {
    market
    quote_collateral_in
    quote_usdc_out
    observed_slot
    bump
}
```

Snapshots are market-specific PDAs.

---

# PDA Model

Osprey uses deterministic PDAs for core protocol state.

Conceptually:

```text
Market
["market", collateral_mint]

Position
["position", ...]

Vault Authority
["vault", ...]

Risk Snapshot
["risk-snapshot", market]
```

These PDAs bind positions, vault authority and risk state to the corresponding lending market.

---

# Meteora DLMM Integration

Osprey integrates with Meteora DLMM's `Swap2` instruction.

The integration constructs the required Meteora instruction data and account layout, including:

- LB pair
- bitmap extension when required
- reserve accounts
- token mints
- token programs
- oracle
- event authority
- memo program
- optional host-fee account
- remaining accounts

The lending vault PDA signs the CPI using `invoke_signed`.

The integration is tested against a mock Meteora program that validates the CPI path and performs a nested Token-2022 transfer into the lending protocol's debt vault.

This tests:

```text
Osprey Liquidate
       │
       ▼
invoke_signed
       │
       ▼
Meteora Swap2 interface
       │
       ▼
Token-2022 transfer
       │
       ▼
Debt Vault
       │
       ▼
Actual Recovery Measurement
```

---

# Why Token-2022 Matters

The target PreStock collateral uses Token-2022 rather than the legacy SPL Token program.

That introduces additional protocol considerations.

The collateral examined for the MVP exposes extensions including:

```text
Transfer Fee
Permanent Delegate
Pausable Configuration
Metadata Pointer
```

These capabilities matter for lending protocols.

For example, a permanent delegate may have authority over tokens held inside protocol-controlled token accounts even though the lending program itself did not authorize a transfer.

This means custody alone does not eliminate issuer risk.

Osprey therefore models issuer risk separately from execution liquidity.

---

# Current Risk Parameters

The MVP currently uses:

| Parameter | Value |
|---|---:|
| Maximum LTV | 65% |
| Minimum LTV | 35% |
| Liquidation Threshold | 70% |
| Liquidation Bonus | 5% |
| Maximum Liquidation | 25% |
| Issuer Risk Ceiling | 60% |
| Reference Liquidation Size | $5,000 |
| PreStocks Transfer Fee | 0.5% |
| Maximum Snapshot Age | 300 slots |

These values are **hackathon MVP parameters**, not production risk recommendations.

Production deployment would require market-specific calibration, monitoring, stress testing, and governance or another controlled risk-management process.

---

# Testing

The current backend test suite contains **58 passing tests** across the protocol.

Coverage includes:

### Risk Engine

- perfect execution
- issuer-risk ceiling
- deteriorating execution liquidity
- minimum-LTV floor
- maximum-LTV cap

### Borrowing

- successful borrowing
- repeated borrowing
- LTV violations
- zero amounts
- insufficient debt liquidity
- incorrect debt mint
- failed-operation state preservation

### Deposits

- successful collateral deposits
- repeated deposits
- Token-2022 transfer-fee accounting
- insufficient balances
- incorrect mint
- zero amounts
- incorrect position PDA

### Withdrawals

- successful withdrawal
- withdrawal with outstanding debt
- effective-LTV enforcement
- excessive withdrawals
- zero amounts
- transfer-fee behavior
- state preservation on failure

### Repayment

- partial repayment
- full repayment
- repeated repayments
- zero amounts
- excessive repayment attempts

### Liquidation

- liquidation eligibility
- liquidation threshold boundaries
- maximum liquidation cap
- collateral valuation
- required debt repayment
- minimum collateral sale calculation
- liquidation bonus calculations
- insufficient execution recovery
- excess execution recovery
- debt-reduction caps
- post-liquidation health validation

### Meteora Integration

- `Swap2` discriminator
- account ordering
- optional accounts
- remaining accounts
- remaining-account serialization
- amount serialization
- event-authority derivation
- end-to-end liquidation CPI through a mock Meteora program

The project currently passes:

```bash
cargo test -p liquidity-aware-lending
```

and:

```bash
anchor build -p liquidity-aware-lending
```

---

# Demo Flow

A simple demonstration of Osprey can follow this lifecycle.

### 1. Initialize a PreStock market

Configure:

```text
Collateral: PreStock
Debt: USDC
Maximum LTV: 65%
Issuer ceiling: 60%
Liquidation threshold: 70%
Reference liquidation: $5,000
```

### 2. Deposit collateral

A borrower deposits Token-2022 PreStock collateral.

Osprey records the actual amount received after any applicable transfer fee.

### 3. Observe Meteora liquidity

Obtain an execution quote for the configured reference liquidation size.

Example:

```text
Reference value:     $5,000
Executable recovery: $4,915
```

### 4. Update the liquidity-risk snapshot

The authorized updater submits the observation on-chain.

Osprey calculates:

```text
Recovery ≈ 98.3%

Execution LTV:
65% × 98.3% ≈ 63.9%

Issuer ceiling:
60%

Effective LTV:
60%
```

### 5. Borrow USDC

Borrowing capacity is enforced using the 60% effective LTV.

### 6. Simulate deteriorating liquidity

Suppose the reference liquidation can now recover only:

```text
$4,000
```

Then:

```text
Recovery = 80%

Execution LTV:
65% × 80% = 52%

Effective LTV:
min(52%, 60%) = 52%
```

The position now has less borrowing capacity even if the quoted collateral price itself has not changed.

### 7. Liquidate an unhealthy position

If the position crosses the liquidation threshold, Osprey can execute the collateral sale through Meteora.

The protocol measures actual USDC returned by the swap and uses the realized recovery when updating the position.

---

# Current MVP Limitations

Osprey is currently a hackathon MVP.

The current implementation intentionally does **not** include:

- multi-collateral positions
- cross-margin
- portfolio-level risk
- interest-rate curves
- interest accrual
- liquidation auctions
- governance
- multi-DEX routing
- permissionless liquidity-oracle updates
- automated keepers
- production-grade price oracles
- dynamic issuer-risk scoring

The current risk updater is trusted.

The updater is expected to construct the liquidity snapshot from the configured reference-sized liquidation quote. Although `quote_collateral_in` is stored in the snapshot, the current execution-LTV calculation primarily evaluates recovered USDC against the configured reference liquidation notional.

This assumption should be enforced or independently verified in a production implementation.

---

# Future Work

A production-oriented version of Osprey could extend the architecture with:

### Verifiable Liquidity Risk

Derive execution liquidity directly from DLMM state or through independently verifiable observations.

### Multi-DEX Liquidation Routing

Evaluate executable liquidity across multiple venues and route liquidation to the best available path.

### Dynamic Reference Sizes

Calculate liquidity risk across multiple liquidation notionals rather than one fixed reference amount.

For example:

```text
$1k
$5k
$10k
$25k
$50k
```

This could produce a more complete liquidity-depth curve.

### Dynamic Issuer Risk

Evaluate Token-2022 administrative capabilities, issuer configuration changes and collateral-specific risk factors.

### Automated Risk Keepers

Continuously observe market liquidity and submit updated risk snapshots.

### Production Price Oracles

Separate reliable collateral pricing from execution-liquidity measurement while combining both inside the lending risk engine.

### Additional Tokenized Assets

Extend beyond PreStocks to other tokenized equities and real-world assets where nominal market value and executable on-chain liquidity may differ materially.

---

# Built With

- **Solana**
- **Anchor**
- **Rust**
- **Token-2022**
- **Meteora DLMM**
- **LiteSVM**

---

# Core Thesis

Osprey is built around a simple idea:

> **Collateral should not be allowed to borrow against value that the protocol cannot realistically recover.**

For liquid assets, price and recoverable value may be close.

For emerging tokenized real-world assets, they may not be.

Osprey makes that difference part of the lending model.
