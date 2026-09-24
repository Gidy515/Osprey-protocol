use anchor_lang::prelude::Pubkey;

use liquidity_aware_lending::{
    constants::{
        LIQUIDATION_BONUS_BPS, LIQUIDATION_THRESHOLD_BPS, MAX_LIQUIDATION_BPS, MAX_LTV_BPS,
    },
    instructions::liquidate::{
        collateral_required_for_debt, collateral_value_usdc, is_liquidatable,
        max_debt_for_collateral, max_liquidation_amount, minimum_collateral_to_sell,
        remaining_debt_capacity, required_debt_repayment, validate_liquidation,
    },
    state::{MarketConfig, Position},
    validate_liquidation_execution,
};

fn test_market() -> MarketConfig {
    MarketConfig {
        collateral_mint: Pubkey::new_unique(),
        debt_mint: Pubkey::new_unique(),
        collateral_price_usdc: 1_000_000,
        dlmm_pool: Pubkey::new_unique(),
        authority: Pubkey::new_unique(),
        max_ltv_bps: MAX_LTV_BPS,
        min_ltv_bps: 3_500,
        liquidation_threshold_bps: LIQUIDATION_THRESHOLD_BPS,
        liquidation_bonus_bps: LIQUIDATION_BONUS_BPS,
        max_liquidation_bps: MAX_LIQUIDATION_BPS,
        reference_liquidation_size_usdc: 5_000_000_000,
        issuer_risk_ceiling_bps: 6_000,
        transfer_fee_bps: 50,
        bump: 0,
    }
}

fn test_position(collateral_amount: u64, debt_amount: u64) -> Position {
    Position {
        owner: Pubkey::new_unique(),
        market: Pubkey::new_unique(),
        collateral_amount,
        debt_amount,
        bump: 0,
    }
}

#[test]
fn test_collateral_value_usdc() {
    let value = collateral_value_usdc(1_000_000_000, 1_000_000, 9)
        .expect("collateral value calculation should succeed");

    assert_eq!(value, 1_000_000);
}

#[test]
fn test_collateral_value_usdc_for_multiple_tokens() {
    let value = collateral_value_usdc(10_000_000_000, 1_000_000, 9)
        .expect("collateral value calculation should succeed");

    assert_eq!(value, 10_000_000);
}

#[test]
fn test_max_debt_for_collateral() {
    let max_debt = max_debt_for_collateral(10_000_000_000, 6_500)
        .expect("max debt calculation should succeed");

    assert_eq!(max_debt, 6_500_000_000);
}

#[test]
fn test_position_is_not_liquidatable_below_threshold() {
    let liquidatable = is_liquidatable(10_000_000, 6_999_000, LIQUIDATION_THRESHOLD_BPS)
        .expect("liquidation check should succeed");

    assert!(!liquidatable);
}

#[test]
fn test_position_is_liquidatable_above_threshold() {
    let liquidatable = is_liquidatable(10_000_000, 7_001_000, LIQUIDATION_THRESHOLD_BPS)
        .expect("liquidation check should succeed");

    assert!(liquidatable);
}

#[test]
fn test_position_at_exact_threshold_is_not_liquidatable() {
    let liquidatable = is_liquidatable(10_000_000, 7_000_000, LIQUIDATION_THRESHOLD_BPS)
        .expect("liquidation check should succeed");

    assert!(!liquidatable);
}

#[test]
fn test_max_liquidation_amount() {
    let max_liquidation = max_liquidation_amount(1_000_000_000, 2_500)
        .expect("max liquidation calculation should succeed");

    assert_eq!(max_liquidation, 250_000_000);
}

#[test]
fn test_collateral_required_for_debt_includes_bonus() {
    let collateral_required =
        collateral_required_for_debt(1_000_000, 1_000_000, 9, LIQUIDATION_BONUS_BPS)
            .expect("collateral requirement calculation should succeed");

    assert_eq!(collateral_required, 1_050_000_000);
}

#[test]
fn test_remaining_debt_capacity() {
    let capacity = remaining_debt_capacity(10_000_000_000, 1_000_000, 9, MAX_LTV_BPS)
        .expect("remaining debt capacity calculation should succeed");

    assert_eq!(capacity, 6_500_000);
}

#[test]
fn test_required_debt_repayment() {
    // $10 collateral.
    // $7.5 debt.
    //
    // Sell $2 collateral.
    //
    // Remaining collateral = $8.
    // Maximum remaining debt at 65% = $5.20.
    //
    // Required repayment:
    // $7.50 - $5.20 = $2.30.

    let required = required_debt_repayment(
        10_000_000_000,
        7_500_000,
        2_000_000_000,
        1_000_000,
        9,
        MAX_LTV_BPS,
    )
    .expect("required repayment calculation should succeed");

    assert_eq!(required, 2_300_000);
}

#[test]
fn test_required_debt_repayment_after_collateral_sale() {
    // $10 collateral.
    // $6 debt.
    //
    // After selling $1 collateral:
    // remaining collateral = $9
    // max debt at 65% = $5.85
    //
    // The position would actually be unhealthy if we sold $1,
    // so repayment is required.

    let required = required_debt_repayment(
        10_000_000_000,
        6_000_000,
        1_000_000_000,
        1_000_000,
        9,
        MAX_LTV_BPS,
    )
    .expect("required repayment calculation should succeed");

    assert_eq!(required, 150_000);
}

#[test]
fn test_minimum_collateral_to_sell_restores_health() {
    let collateral_amount = 10_000_000_000;
    let debt_amount = 7_000_000;

    // Deterministic quote abstraction:
    //
    // 1 collateral token
    // = 1,000,000,000 collateral base units
    //
    // produces
    //
    // 1 USDC
    // = 1,000,000 USDC base units.
    //
    // This represents a 1:1 execution price.
    let quote_collateral_in = 1_000_000_000;
    let quote_usdc_out = 1_000_000;

    let liquidation_amount = minimum_collateral_to_sell(
        collateral_amount,
        debt_amount,
        1_000_000,
        9,
        MAX_LTV_BPS,
        MAX_LIQUIDATION_BPS,
        quote_collateral_in,
        quote_usdc_out,
    )
    .expect("minimum liquidation calculation should succeed");

    assert!(liquidation_amount > 0);

    assert!(
        liquidation_amount
            <= max_liquidation_amount(collateral_amount, MAX_LIQUIDATION_BPS).unwrap()
    );

    // The resulting position must satisfy the maximum LTV.
    let usdc_recovered = liquidation_amount
        .checked_mul(quote_usdc_out)
        .unwrap()
        .checked_div(quote_collateral_in)
        .unwrap();

    let remaining_collateral = collateral_amount.checked_sub(liquidation_amount).unwrap();

    let remaining_debt = debt_amount.saturating_sub(usdc_recovered);

    let remaining_capacity =
        remaining_debt_capacity(remaining_collateral, 1_000_000, 9, MAX_LTV_BPS).unwrap();

    assert!(
        remaining_debt <= remaining_capacity,
        "liquidation must restore the position to max LTV"
    );

    // Verify that one unit less would NOT have been sufficient.
    if liquidation_amount > 1 {
        let smaller_amount = liquidation_amount - 1;

        let smaller_usdc_recovered = smaller_amount
            .checked_mul(quote_usdc_out)
            .unwrap()
            .checked_div(quote_collateral_in)
            .unwrap();

        let smaller_remaining_collateral = collateral_amount.checked_sub(smaller_amount).unwrap();

        let smaller_remaining_debt = debt_amount.saturating_sub(smaller_usdc_recovered);

        let smaller_capacity =
            remaining_debt_capacity(smaller_remaining_collateral, 1_000_000, 9, MAX_LTV_BPS)
                .unwrap();

        assert!(
            smaller_remaining_debt > smaller_capacity,
            "liquidation amount should be the minimum amount required"
        );
    }
}

#[test]
fn test_minimum_collateral_to_sell_rejects_zero_quote_amounts() {
    let zero_collateral_quote = minimum_collateral_to_sell(
        10_000_000_000,
        7_000_000,
        1_000_000,
        9,
        MAX_LTV_BPS,
        MAX_LIQUIDATION_BPS,
        0,
        1_000_000,
    );

    assert!(zero_collateral_quote.is_err());

    let zero_usdc_quote = minimum_collateral_to_sell(
        10_000_000_000,
        7_000_000,
        1_000_000,
        9,
        MAX_LTV_BPS,
        MAX_LIQUIDATION_BPS,
        1_000_000_000,
        0,
    );

    assert!(zero_usdc_quote.is_err());
}

#[test]
fn test_validate_liquidation_rejects_healthy_position() {
    let market = test_market();

    let position = test_position(10_000_000_000, 6_000_000);

    let result = validate_liquidation(&market, &position, 9);

    assert!(result.is_err(), "healthy position must not be liquidatable");
}

#[test]
fn test_validate_liquidation_accepts_unhealthy_position() {
    let market = test_market();

    let position = test_position(10_000_000_000, 7_500_000);

    validate_liquidation(&market, &position, 9).expect("unhealthy position should be liquidatable");
}

#[test]
fn test_validate_liquidation_rejects_zero_collateral() {
    let market = test_market();

    let position = test_position(0, 1_000_000);

    let result = validate_liquidation(&market, &position, 9);

    assert!(result.is_err());
}

#[test]
fn test_validate_liquidation_rejects_zero_debt() {
    let market = test_market();

    let position = test_position(10_000_000_000, 0);

    let result = validate_liquidation(&market, &position, 9);

    assert!(result.is_err());
}

#[test]
fn test_liquidation_cap_is_25_percent() {
    let collateral = 1_000_000_000;

    let maximum = max_liquidation_amount(collateral, MAX_LIQUIDATION_BPS)
        .expect("liquidation cap calculation should succeed");

    assert_eq!(maximum, 250_000_000);
}

#[test]
fn test_validate_liquidation_execution_success() {
    let collateral_amount = 10_000_000_000;
    let debt_amount = 7_000_000;

    let collateral_to_sell = 2_000_000_000;

    // With $8 remaining collateral:
    // remaining capacity = $5.20
    // required repayment = $2.30
    let usdc_recovered = 2_300_000;

    let debt_reduction = validate_liquidation_execution(
        collateral_amount,
        debt_amount,
        collateral_to_sell,
        usdc_recovered,
        1_000_000,
        9,
        6_500,
        2_500,
    )
    .unwrap();

    assert_eq!(debt_reduction, 2_300_000);
}

#[test]
fn test_validate_liquidation_execution_allows_excess_recovery() {
    let collateral_amount = 10_000_000_000;
    let debt_amount = 7_000_000;

    let collateral_to_sell = 2_000_000_000;

    // Required repayment is $2.30, but the actual swap recovered $2.50.
    let usdc_recovered = 2_500_000;

    let debt_reduction = validate_liquidation_execution(
        collateral_amount,
        debt_amount,
        collateral_to_sell,
        usdc_recovered,
        1_000_000,
        9,
        6_500,
        2_500,
    )
    .unwrap();

    assert_eq!(debt_reduction, 2_500_000);
}

#[test]
fn test_validate_liquidation_execution_rejects_insufficient_recovery() {
    let collateral_amount = 10_000_000_000;
    let debt_amount = 7_000_000;

    let collateral_to_sell = 2_000_000_000;

    // With $8 remaining collateral at 65% max LTV:
    // remaining debt capacity = $5.20
    // required repayment = $7.00 - $5.20 = $1.80
    //
    // Only $1.50 was recovered, so the position cannot be restored
    // to the required health level.
    let usdc_recovered = 1_500_000;

    let result = validate_liquidation_execution(
        collateral_amount,
        debt_amount,
        collateral_to_sell,
        usdc_recovered,
        1_000_000,
        9,
        6_500,
        2_500,
    );

    assert!(result.is_err());
}

#[test]
fn test_validate_liquidation_execution_rejects_cap_breach() {
    let collateral_amount = 10_000_000_000;
    let debt_amount = 7_000_000;

    // 30% of collateral, above the configured 25% cap.
    let collateral_to_sell = 3_000_000_000;

    let result = validate_liquidation_execution(
        collateral_amount,
        debt_amount,
        collateral_to_sell,
        5_000_000,
        1_000_000,
        9,
        6_500,
        2_500,
    );

    assert!(result.is_err());
}

#[test]
fn test_validate_liquidation_execution_rejects_zero_collateral_sale() {
    let result = validate_liquidation_execution(
        10_000_000_000,
        7_000_000,
        0,
        2_300_000,
        1_000_000,
        9,
        6_500,
        2_500,
    );

    assert!(result.is_err());
}

#[test]
fn test_validate_liquidation_execution_caps_debt_reduction() {
    let collateral_amount = 10_000_000_000;
    let debt_amount = 7_000_000;

    let collateral_to_sell = 2_000_000_000;

    // More USDC recovered than the entire debt.
    let usdc_recovered = 10_000_000;

    let debt_reduction = validate_liquidation_execution(
        collateral_amount,
        debt_amount,
        collateral_to_sell,
        usdc_recovered,
        1_000_000,
        9,
        6_500,
        2_500,
    )
    .unwrap();

    assert_eq!(debt_reduction, debt_amount);
}
