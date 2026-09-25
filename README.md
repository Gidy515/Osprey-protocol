# Osprey Protocol

### Liquidity-aware credit infrastructure for tokenized equities.

Osprey is a lending protocol designed to turn tokenized equities into productive onchain collateral.

As equities and private-market assets move onchain, simply tokenizing them is not enough. For these assets to become useful across DeFi, they need credit infrastructure that understands a fundamental difference between **quoted asset value** and **value that can actually be recovered onchain**.

Osprey addresses this by making **executable liquidity a first-class input to lending risk**.

Instead of determining borrowing capacity only from a static loan-to-value ratio, Osprey evaluates the liquidity available to liquidate an asset, combines it with issuer-specific risk constraints, and enforces the resulting borrowing capacity onchain.

```text id="e6bqf2"
Tokenized Equities
        │
        ▼
 Onchain Liquidity
        │
        ▼
   Osprey Risk Engine
        │
        ▼
Liquidity-Adjusted Credit
        │
        ▼
     DeFi Users
```

The long-term opportunity is to build a credit layer for the growing tokenized-equity economy.

---

# The Opportunity

Capital markets are moving onchain.

Tokenized public equities, pre-IPO exposure, private-company shares, funds, and other real-world financial assets are increasingly becoming programmable blockchain assets.

But tokenization alone does not create a financial ecosystem.

For tokenized equities to become deeply integrated into DeFi, holders need to be able to:

- borrow against their positions;
- access stablecoin liquidity without selling;
- use equity exposure as productive collateral;
- move capital between traditional-market exposure and crypto-native markets;
- build leveraged and structured positions around tokenized assets.

This creates an opportunity for a new category of lending infrastructure purpose-built for tokenized assets.

Osprey is designed around that opportunity.

---

# The Problem With Traditional LTV

Most lending protocols begin with a relatively simple model:

```text id="2zhqxb"
Collateral Value
      ×
    LTV
      ↓
Borrowing Capacity
```

If an asset is worth `$10,000` and has a `65%` maximum LTV, a protocol may allow approximately `$6,500` of borrowing.

But this assumes that the collateral can actually be liquidated close to its quoted value.

That assumption becomes increasingly dangerous for emerging tokenized equities.

A position may have:

```text id="6ig7d7"
Quoted market value:     $10,000

But...

Executable DEX value:     $8,900
```

The difference can come from:

- limited secondary-market liquidity;
- fragmented liquidity;
- price impact;
- slippage;
- transfer fees;
- market concentration;
- issuer controls;
- administrative properties of the token.

The question therefore should not only be:

> **What is this collateral worth?**

It should also be:

> **What can the protocol realistically recover if this position must be liquidated?**

That is the problem Osprey is built around.

---

# Liquidity-Aware Lending

Osprey separates collateral risk into two major components:

```text id="2i0fjo"
                 COLLATERAL RISK
                       │
          ┌────────────┴────────────┐
          │                         │
    Execution Risk              Issuer Risk
          │                         │
          ▼                         ▼
Executable liquidation      Asset / issuer controls
liquidity onchain           and administrative risk
          │                         │
          ▼                         ▼
 Execution-derived LTV      Issuer-risk ceiling
          │                         │
          └────────────┬────────────┘
                       ▼
                  Effective LTV
```

The resulting borrowing constraint is:

```text id="pajh4x"
effective LTV =
min(
    execution-derived LTV,
    issuer-risk ceiling
)
```

This creates a lending market that can respond to the actual liquidity characteristics of its collateral.

---

# How It Works

Osprey evaluates a reference liquidation against the asset's onchain liquidity.

For the current market configuration:

```text id="7d8fph"
Maximum LTV:                  65%
Minimum LTV:                  35%
Issuer-risk ceiling:          60%
Reference liquidation:     $5,000
```

Suppose `$5,000` worth of tokenized equity can currently recover only `$4,495.56` through available DEX liquidity.

```text id="9nkgbp"
Reference value
$5,000
   │
   ▼
DEX execution
   │
   ▼
$4,495.56 recoverable
   │
   ▼
89.91% recovery
```

Osprey adjusts the maximum LTV accordingly:

```text id="vab9w3"
65% maximum LTV
       ×
89.91% liquidity recovery
       ↓
58.44% execution-derived LTV
```

The protocol then applies the issuer-risk ceiling:

```text id="w6bdfd"
Execution-derived LTV     58.44%
Issuer-risk ceiling       60.00%
                           │
                           ▼
Effective LTV             58.44%
```

Borrowing capacity contracts from `65%` to `58.44%`.

The important distinction is that this is not merely a frontend risk indicator.

**The effective LTV is enforced by the lending program.**

---

# Osprey's Position in the Tokenized-Equity Stack

Osprey is not an equity issuer.

It is credit infrastructure built around tokenized equities issued by other platforms.

```text id="kxy5v9"
┌─────────────────────────────────────┐
│       Tokenized Equity Issuers      │
│                                     │
│ Public / Private / Pre-IPO Exposure │
└──────────────────┬──────────────────┘
                   │
                   ▼
┌─────────────────────────────────────┐
│       Onchain Liquidity Venues      │
│                                     │
│       DEXs / Market Makers          │
└──────────────────┬──────────────────┘
                   │
                   ▼
┌─────────────────────────────────────┐
│               OSPREY                │
│                                     │
│ Liquidity-Aware Risk                │
│ Collateral Management               │
│ Borrowing                            │
│ Liquidation                         │
└──────────────────┬──────────────────┘
                   │
                   ▼
┌─────────────────────────────────────┐
│             DeFi Users              │
│                                     │
│ Stablecoin Liquidity                │
│ Capital Efficiency                  │
│ Onchain Credit                      │
└─────────────────────────────────────┘
```

This means Osprey can potentially support multiple tokenized-equity issuers without becoming the issuer itself.

---

# Who Osprey Is For

## Tokenized-Equity Holders

A holder may want exposure to an equity without selling the position whenever liquidity is needed.

Osprey can allow that asset to become productive collateral.

```text id="33t3cn"
Tokenized equity
       │
       ▼
Deposit into Osprey
       │
       ▼
Borrow stablecoins
       │
       ▼
Deploy capital elsewhere
```

The user maintains the collateral position while accessing onchain liquidity.

---

## DeFi Users

Crypto-native users already borrow against assets such as SOL, ETH, BTC, and liquid staking tokens.

As traditional financial assets move onchain, tokenized equities create another collateral category.

Osprey provides infrastructure for bringing that collateral into DeFi lending markets while accounting for characteristics that differ from highly liquid crypto assets.

---

## Tokenized-Asset Issuers

Issuers benefit when their assets become useful beyond spot trading.

A tokenized asset that can participate in:

```text id="9qj95k"
Trading
   +
Collateral
   +
Credit
   +
Structured Products
```

has substantially more potential utility than an asset restricted to buy-and-hold exposure.

Osprey can provide lending infrastructure without requiring the issuer to build an entire credit protocol internally.

---

## Liquidity Providers

Liquidity becomes economically important beyond trading volume.

In Osprey's model, deeper executable liquidity can directly improve the collateral efficiency of an asset.

Conceptually:

```text id="91wb47"
Deeper liquidity
      ↓
Better liquidation recovery
      ↓
Higher execution-derived LTV
      ↓
Greater borrowing capacity
```

Liquidity provision therefore becomes part of the credit infrastructure surrounding the tokenized asset.

---

# The Strategic Flywheel

Osprey's longer-term opportunity is a feedback loop between tokenization, liquidity, and credit.

```text id="t3f4i6"
More tokenized assets
        │
        ▼
More onchain liquidity
        │
        ▼
Better liquidation execution
        │
        ▼
Greater collateral efficiency
        │
        ▼
More borrowing activity
        │
        ▼
Greater asset utility
        │
        └───────────────┐
                        │
                        ▼
               More tokenized assets
```

This creates a potential infrastructure layer connecting tokenized capital markets with crypto-native credit.

---

# Protocol Architecture

```text id="d91l3v"
┌───────────────────────────────────────────────┐
│                 Osprey App                    │
│                                               │
│ Wallet │ Positions │ Risk │ Lending Actions   │
└───────────────────────┬───────────────────────┘
                        │
                        ▼
┌───────────────────────────────────────────────┐
│              Osprey Protocol                  │
│                                               │
│ MarketConfig                                  │
│ Position                                      │
│ LiquidityRiskSnapshot                         │
│                                               │
│ ┌───────────────────────────────────────────┐ │
│ │              Risk Engine                  │ │
│ │                                           │ │
│ │ Recovery                                  │ │
│ │    ↓                                      │ │
│ │ Execution LTV                             │ │
│ │    ↓                                      │ │
│ │ Issuer Ceiling                            │ │
│ │    ↓                                      │ │
│ │ Effective LTV                             │ │
│ └───────────────────────────────────────────┘ │
│                                               │
│ Deposit │ Borrow │ Repay │ Withdraw           │
│ Liquidate │ Update Liquidity Risk             │
└───────────────────┬───────────────────────────┘
                    │
              ┌─────┴─────┐
              ▼           ▼
         Token-2022    Meteora DLMM
            Vaults     Liquidity Layer
```

---

# Core Protocol Operations

| Instruction | Function |
|---|---|
| `initialize` | Creates an isolated tokenized-equity lending market |
| `deposit_collateral` | Deposits collateral into the protocol |
| `borrow` | Borrows stablecoins against available collateral capacity |
| `repay` | Reduces a position's outstanding debt |
| `withdraw_collateral` | Withdraws collateral while maintaining position health |
| `liquidate` | Liquidates unhealthy collateral |
| `update_liquidity_risk` | Updates executable-liquidity information used by the risk engine |

---

# Executable Liquidity Through Meteora

The current implementation uses Meteora DLMM as its executable-liquidity layer.

Osprey uses the market for two different purposes.

## Risk Discovery

A reference amount of collateral is quoted against its stablecoin market.

```text id="w3qb9n"
Collateral
    │
    ▼
Meteora DLMM
    │
    ▼
Executable output
    │
    ▼
Recovery ratio
    │
    ▼
Execution-derived LTV
```

This gives the lending protocol information about the depth available behind the quoted collateral value.

## Liquidation

Osprey also integrates with Meteora's `Swap2` instruction for liquidation execution.

```text id="5gtv95"
Unhealthy position
       │
       ▼
Collateral vault
       │
       ▼
Meteora DLMM
       │
       ▼
Actual stablecoins recovered
       │
       ▼
Debt reduction
```

The program measures the actual stablecoin balance recovered during execution rather than assuming the quoted amount was received.

---

# Dynamic Risk Protection

Liquidity can change much faster than traditional collateral parameters.

Osprey therefore stores liquidity observations onchain:

```text id="fqckdp"
LiquidityRiskSnapshot {
    market
    quote_collateral_in
    quote_usdc_out
    observed_slot
}
```

Snapshots expire after a configured period.

The current implementation uses a maximum age of:

```text id="ibnm9v"
300 Solana slots
```

Once liquidity information becomes stale, risk-increasing actions are blocked.

```text id="w63jqs"
                  Fresh        Stale

Deposit             ✓            ✓
Borrow              ✓            ✕
Repay               ✓            ✓
Withdraw            ✓            ✕
```

This creates an important asymmetry:

**Users can continue reducing protocol risk even when liquidity information is stale, but cannot increase exposure using outdated assumptions.**

---

# Token-2022 and Issuer Risk

Tokenized assets can have properties that conventional crypto collateral does not.

Examples include:

- transfer fees;
- freeze authority;
- permanent delegates;
- administrative controls;
- issuer-specific token configuration.

Osprey separates these concerns from execution liquidity through an **issuer-risk ceiling**.

```text id="bbnb4h"
Excellent DEX liquidity
         │
         ▼
65% execution LTV

But issuer ceiling = 60%
         │
         ▼
Effective LTV = 60%
```

This prevents deep secondary-market liquidity from automatically overriding other risks associated with the asset.

The protocol also accounts for Token-2022 transfer-fee behavior by measuring actual collateral received by its vault rather than assuming the requested transfer amount was received.

---

# Market Structure

Osprey uses isolated lending markets.

Each market defines its own:

```text id="e42xv5"
Collateral asset
Debt asset
DEX liquidity venue
Maximum LTV
Minimum LTV
Liquidation threshold
Liquidation bonus
Liquidation cap
Issuer-risk ceiling
Reference liquidation size
Transfer-fee configuration
```

This is particularly relevant for tokenized equities because different assets can have radically different:

- liquidity profiles;
- issuers;
- market depth;
- administrative controls;
- trading activity.

Risk therefore does not need to be generalized across every supported equity.

---

# Business Opportunity

Osprey can evolve into infrastructure serving three sides of the tokenized-asset market:

```text id="h8yk2u"
               OSPREY
                  │
       ┌──────────┼──────────┐
       │          │          │
       ▼          ▼          ▼
    Issuers    Borrowers   Liquidity
                           Providers
```

Potential protocol activity can come from:

- interest paid by borrowers;
- liquidation fees;
- market creation and risk infrastructure;
- integrations with tokenized-asset issuers;
- institutional or permissioned lending markets;
- future structured credit products.

The current implementation focuses on proving the underlying lending and risk architecture before introducing a full interest-rate and protocol-fee model.

---

# Expansion Strategy

The current design establishes the primitive required for a broader tokenized-equity credit network.

### Phase 1 — Isolated Equity Lending

```text id="fnm3rq"
Tokenized Equity
       ↓
Stablecoin Credit
```

Launch isolated markets for individual tokenized assets with liquidity-aware collateral parameters.

### Phase 2 — Multi-Venue Liquidity

Instead of relying on one liquidity venue:

```text id="kjmgp2"
             Asset
               │
       ┌───────┼───────┐
       ▼       ▼       ▼
     DEX A   DEX B   DEX C
       │       │       │
       └───────┼───────┘
               ▼
        Liquidity Engine
               ▼
        Effective LTV
```

Osprey can evaluate executable liquidity across multiple markets.

### Phase 3 — Tokenized-Equity Credit Markets

Additional credit primitives can be built on top of the risk engine:

- variable-rate lending;
- institutional markets;
- structured lending products;
- portfolio collateral;
- tokenized-equity credit vaults;
- automated liquidity-risk monitoring.

### Phase 4 — Cross-Market Collateral Infrastructure

The broader goal is for Osprey's liquidity-aware risk infrastructure to become reusable wherever tokenized assets are accepted as collateral.

---

# Current Implementation

The current protocol already implements the core lending lifecycle:

```text id="ht6f1q"
Initialize Market
       ↓
Observe DEX Liquidity
       ↓
Update Risk Snapshot
       ↓
Deposit Collateral
       ↓
Borrow Stablecoins
       ↓
Repay Debt
       ↓
Withdraw Collateral
```

The program additionally contains liquidation logic and Meteora CPI integration.

---

# Devnet Deployment

## Osprey Program

```text id="s32zzu"
jAUJs14M4WiAvrgRgZiKQ2nunkWMrqqM7sGZotmWwss
```

## ANTH / USDC Market

```text id="hs6eg8"
Market
Dsci5yxTLxHF6332xRc4gMEEsZxqjGMdQdv1GJ8WvNG

Liquidity Risk Snapshot
2Nx8YXGyVUSceNcP1YgH141T5BS24QQqJxVPsSXjpvSf

Vault Authority
BkCD6v4wQAH1TMCEuUxBx2fdDnZiQd3CRp9k6KjZw79X
```

## Devnet Assets

```text id="hz9rtf"
ANTH representation
ArzQGNtfXXQSLcdtVrJPu1CuJZHTiejL55hfum35FsKv

USDC test asset
9nPD667QBEV9rzmySdAqttPM5aTQWkjFH3R6JLhhF734
```

## Meteora DLMM

```text id="4xxyr4"
53vdaEoUXhhTXGM9odCYVdvGYzLJnSMXovX5MX9E3xnU
```

---

# Devnet Asset Note

The current devnet collateral is a **Meteora-compatible Token-2022 test asset representing ANTH and used to exercise the complete lending architecture**.

It is not an official Anthropic asset, and Osprey is not affiliated with or endorsed by Anthropic.

The production risk model is designed to account for issuer controls such as permanent-delegate and freeze authority.

During development, the tested devnet Meteora deployment rejected the PreStock-like Token-2022 extension combination used in the initial fixture. The current devnet asset therefore uses a simplified Token-2022 configuration while preserving transfer-fee behavior.

This allows the deployed market to exercise Osprey's liquidity-aware credit mechanism without representing the devnet collateral as an actual tokenized equity.

---

# Current Trust Model

The current liquidity-risk snapshot is:

**Meteora-derived, authority-updated, and onchain-enforced.**

An authorized market operator obtains executable-liquidity information and updates the market's onchain risk snapshot.

The program independently enforces:

- snapshot freshness;
- effective LTV;
- borrowing limits;
- withdrawal health;
- liquidation constraints.

The current updater should not be interpreted as a trustless oracle.

Future versions can decentralize this layer through permissionless verification, multiple liquidity sources, or dedicated oracle infrastructure.

---

# Repository

```text id="y88gs2"
.
├── app/
│   ├── scripts/
│   └── src/
│       ├── components/
│       └── lib/
│           ├── anchor/
│           ├── osprey/
│           └── solana/
│
├── programs/
│   ├── liquidity-aware-lending/
│   └── mock-meteora/
│
├── tests/
├── Anchor.toml
├── Cargo.toml
└── README.md
```

---

# Running Osprey

## Build the protocol

```bash id="oqn2wl"
anchor build -p liquidity-aware-lending
```

## Run tests

```bash id="ucaqyy"
cargo test -p liquidity-aware-lending
```

## Run the application

```bash id="0n8jcs"
cd app
yarn install
yarn dev
```

## Quote current liquidity

```bash id="ypom3l"
cd app
yarn tsx scripts/quote-meteora-risk.ts
```

## Refresh the risk snapshot

```bash id="8if9l2"
yarn tsx scripts/refresh-osprey-risk.ts
```

---

# Current Scope

The current implementation focuses on validating Osprey's core thesis:

> **Onchain borrowing capacity for tokenized assets should reflect the liquidity available to actually exit those assets.**

Implemented today:

- isolated lending markets;
- Token-2022 collateral;
- stablecoin borrowing;
- repayment;
- collateral withdrawals;
- liquidity-derived LTV;
- issuer-risk ceilings;
- stale-liquidity protection;
- capped liquidation;
- Meteora DLMM integration;
- onchain position accounting;
- web application;
- Solana devnet deployment.

Future development expands the credit layer rather than changing this fundamental model.

---

# Vision

Tokenization puts financial assets onchain.

The next step is making those assets **financially useful once they get there**.

Osprey's thesis is that tokenized equities will require more than issuance and spot markets. They will need lending, collateral management, liquidation infrastructure, risk engines, and eventually an entire credit layer connecting traditional asset exposure with crypto-native capital.

Osprey is building toward that layer.

**Tokenized equities become collateral.  
Liquidity becomes risk data.  
Collateral becomes credit.**
