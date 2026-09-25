use anchor_lang::prelude::*;

use crate::{
    constants::MAX_RISK_SNAPSHOT_AGE_SLOTS,
    error::LendingError,
    state::{LiquidityRiskSnapshot, MarketConfig},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RiskAssessment {
    /// LTV implied by executable Meteora liquidity.
    pub execution_ltv_bps: u16,

    /// Final protocol LTV after issuer/admin-risk ceiling.
    pub effective_ltv_bps: u16,

    /// Maximum debt allowed for the supplied collateral value.
    pub max_debt: u64,
}

/// Ensure the liquidity observation is recent enough to use.
pub fn validate_snapshot_freshness(
    snapshot: &LiquidityRiskSnapshot,
    current_slot: u64,
) -> Result<()> {
    require!(
        current_slot >= snapshot.observed_slot,
        LendingError::StaleLiquidityQuote
    );

    let age = current_slot
        .checked_sub(snapshot.observed_slot)
        .ok_or(LendingError::MathOverflow)?;

    require!(
        age <= MAX_RISK_SNAPSHOT_AGE_SLOTS,
        LendingError::StaleLiquidityQuote
    );

    Ok(())
}

/// Convert executable liquidation output into an execution-derived LTV.
///
/// The market defines a reference liquidation notional in USDC.
///
/// Example:
///
/// reference size = $5,000
/// executable output = $4,900
///
/// recovery = 98%
///
/// execution_ltv = configured max LTV * 98%
///
/// The result is clamped between the market's configured min/max LTV.
pub fn execution_ltv_from_quote(market: &MarketConfig, quote_usdc_out: u64) -> Result<u16> {
    require!(
        market.reference_liquidation_size_usdc > 0,
        LendingError::InvalidMarketConfig
    );

    require!(quote_usdc_out > 0, LendingError::InvalidAmount);

    let capped_output = quote_usdc_out.min(market.reference_liquidation_size_usdc);

    let recovery_bps = capped_output
        .checked_mul(10_000)
        .ok_or(LendingError::MathOverflow)?
        .checked_div(market.reference_liquidation_size_usdc)
        .ok_or(LendingError::MathOverflow)?;

    let raw_execution_ltv = (market.max_ltv_bps as u64)
        .checked_mul(recovery_bps)
        .ok_or(LendingError::MathOverflow)?
        .checked_div(10_000)
        .ok_or(LendingError::MathOverflow)?;

    let clamped = raw_execution_ltv
        .max(market.min_ltv_bps as u64)
        .min(market.max_ltv_bps as u64);

    u16::try_from(clamped).map_err(|_| LendingError::MathOverflow.into())
}

/// Apply issuer/admin risk after execution risk.
///
/// final LTV = min(execution-derived LTV, issuer-risk ceiling)
pub fn effective_ltv_bps(market: &MarketConfig, quote_usdc_out: u64) -> Result<u16> {
    let execution_ltv = execution_ltv_from_quote(market, quote_usdc_out)?;

    Ok(execution_ltv.min(market.issuer_risk_ceiling_bps))
}

/// Complete risk assessment for a collateral position.
pub fn assess_risk(
    market: &MarketConfig,
    snapshot: &LiquidityRiskSnapshot,
    collateral_value_usdc: u64,
    current_slot: u64,
) -> Result<RiskAssessment> {
    require!(
        snapshot.quote_collateral_in > 0,
        LendingError::InvalidAmount
    );

    validate_snapshot_freshness(snapshot, current_slot)?;

    let execution_ltv_bps = execution_ltv_from_quote(market, snapshot.quote_usdc_out)?;

    let effective_ltv_bps = execution_ltv_bps.min(market.issuer_risk_ceiling_bps);

    let max_debt_u128 = (collateral_value_usdc as u128)
        .checked_mul(effective_ltv_bps as u128)
        .ok_or(LendingError::MathOverflow)?
        .checked_div(10_000u128)
        .ok_or(LendingError::MathOverflow)?;

    let max_debt =
        u64::try_from(max_debt_u128)
            .map_err(|_| LendingError::MathOverflow)?;

    Ok(RiskAssessment {
        execution_ltv_bps,
        effective_ltv_bps,
        max_debt,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn market() -> MarketConfig {
        MarketConfig {
            collateral_mint: Pubkey::new_unique(),
            debt_mint: Pubkey::new_unique(),
            collateral_price_usdc: 1_000_000,
            dlmm_pool: Pubkey::new_unique(),
            authority: Pubkey::new_unique(),
            max_ltv_bps: 6_500,
            min_ltv_bps: 3_500,
            liquidation_threshold_bps: 7_000,
            liquidation_bonus_bps: 500,
            max_liquidation_bps: 2_500,
            reference_liquidation_size_usdc: 5_000_000_000,
            issuer_risk_ceiling_bps: 6_000,
            transfer_fee_bps: 50,
            bump: 0,
        }
    }

    #[test]
    fn perfect_execution_is_capped_by_issuer_risk() {
        let market = market();

        let execution = execution_ltv_from_quote(&market, 5_000_000_000).unwrap();

        let effective = effective_ltv_bps(&market, 5_000_000_000).unwrap();

        assert_eq!(execution, 6_500);
        assert_eq!(effective, 6_000);
    }

    #[test]
    fn execution_loss_reduces_ltv() {
        let market = market();

        // 90% executable recovery.
        let execution = execution_ltv_from_quote(&market, 4_500_000_000).unwrap();

        assert_eq!(execution, 5_850);
    }

    #[test]
    fn execution_ltv_cannot_fall_below_minimum() {
        let market = market();

        let execution = execution_ltv_from_quote(&market, 1_000_000_000).unwrap();

        assert_eq!(execution, 3_500);
    }

    #[test]
    fn quote_above_reference_cannot_increase_max_ltv() {
        let market = market();

        let execution = execution_ltv_from_quote(&market, 6_000_000_000).unwrap();

        assert_eq!(execution, 6_500);
    }
}
