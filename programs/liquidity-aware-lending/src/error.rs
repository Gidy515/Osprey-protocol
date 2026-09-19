use anchor_lang::prelude::*;

#[error_code]
pub enum LendingError {
    #[msg("Invalid market configuration")]
    InvalidMarketConfig,

    #[msg("Collateral mint does not match the market")]
    MarketMismatch,

    #[msg("Debt mint does not match the market")]
    DebtMintMismatch,

    #[msg("Invalid LTV configuration")]
    InvalidLtvConfiguration,

    #[msg("Invalid amount")]
    InvalidAmount,

    #[msg("Insufficient collateral")]
    InsufficientCollateral,

    #[msg("Borrow amount exceeds allowed LTV")]
    BorrowExceedsLtv,

    #[msg("Withdrawal would violate the required LTV")]
    WithdrawalViolatesLtv,

    #[msg("Position is unhealthy")]
    UnhealthyPosition,

    #[msg("Position is healthy and cannot be liquidated")]
    PositionHealthy,

    #[msg("Requested liquidation exceeds the configured cap")]
    LiquidationExceedsCap,

    #[msg("Liquidity quote is stale")]
    StaleLiquidityQuote,

    #[msg("Liquidation could not restore the position to a healthy state")]
    LiquidationCannotRestoreHealth,

    #[msg("Issuer risk ceiling has been breached")]
    IssuerRiskCeilingBreached,

    #[msg("Insufficient debt liquidity")]
    InsufficientDebtLiquidity,

    #[msg("Arithmetic overflow")]
    MathOverflow,
}
