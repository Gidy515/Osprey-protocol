use anchor_lang::prelude::*;

#[account]
pub struct MarketConfig {
    /// PreStocks collateral mint.
    pub collateral_mint: Pubkey,

    /// Debt mint, currently USDC.
    pub debt_mint: Pubkey,

    /// MVP reference price of one whole collateral token
    /// denominated in USDC base units.
    pub collateral_price_usdc: u64,

    /// Meteora DLMM pool used for liquidity/risk
    /// assessment and liquidation execution.
    pub dlmm_pool: Pubkey,

    /// Authority allowed to initialize/configure the market.
    pub authority: Pubkey,

    /// Absolute protocol LTV ceiling.
    pub max_ltv_bps: u16,

    /// Floor for the execution-risk LTV model.
    pub min_ltv_bps: u16,

    /// Position becomes liquidatable at this LTV.
    pub liquidation_threshold_bps: u16,

    /// Incentive paid to the liquidator.
    pub liquidation_bonus_bps: u16,

    /// Maximum percentage of collateral that can be
    /// liquidated in one transaction.
    pub max_liquidation_bps: u16,

    /// Standardized liquidation notional, denominated
    /// in USDC base units.
    pub reference_liquidation_size_usdc: u64,

    /// Independent ceiling caused by issuer/admin risk.
    pub issuer_risk_ceiling_bps: u16,

    /// Expected Token-2022 transfer fee.
    pub transfer_fee_bps: u16,

    /// Market PDA bump.
    pub bump: u8,
}

impl MarketConfig {
    pub const LEN: usize = 8 +  // discriminator
        32 + // collateral_mint
        32 + // debt_mint
        8 +  // collateral_price_usdc
        32 + // dlmm_pool
        32 + // authority
        2 +  // max_ltv_bps
        2 +  // min_ltv_bps
        2 +  // liquidation_threshold_bps
        2 +  // liquidation_bonus_bps
        2 +  // max_liquidation_bps
        8 +  // reference_liquidation_size_usdc
        2 +  // issuer_risk_ceiling_bps
        2 +  // transfer_fee_bps
        1; // bump
}

#[account]
pub struct Position {
    /// Wallet controlling this borrowing position.
    pub owner: Pubkey,

    /// Market this position belongs to.
    pub market: Pubkey,

    /// Collateral deposited into the market.
    pub collateral_amount: u64,

    /// Outstanding debt.
    pub debt_amount: u64,

    /// Position PDA bump.
    pub bump: u8,
}

impl Position {
    pub const LEN: usize = 8 +  // discriminator
        32 + // owner
        32 + // market
        8 +  // collateral_amount
        8 +  // debt_amount
        1; // bump
}

#[account]
pub struct LiquidityRiskSnapshot {
    /// Market this snapshot belongs to.
    pub market: Pubkey,

    /// Collateral amount used for the executable-liquidity quote.
    pub quote_collateral_in: u64,

    /// Executable USDC output observed from Meteora.
    pub quote_usdc_out: u64,

    /// Slot at which the observation was published.
    pub observed_slot: u64,

    /// PDA bump.
    pub bump: u8,
}

impl LiquidityRiskSnapshot {
    pub const LEN: usize = 8 +  // discriminator
        32 + // market
        8 +  // quote_collateral_in
        8 +  // quote_usdc_out
        8 +  // observed_slot
        1; // bump
}
