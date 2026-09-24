use anchor_lang::prelude::*;

use crate::{
    constants::*,
    error::LendingError,
    risk_engine::execution_ltv_from_quote,
    state::{LiquidityRiskSnapshot, MarketConfig},
};

#[derive(Accounts)]
pub struct UpdateLiquidityRisk<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        seeds = [MARKET_SEED, market.collateral_mint.as_ref()],
        bump = market.bump,
        has_one = authority @ LendingError::InvalidMarketConfig,
    )]
    pub market: Box<Account<'info, MarketConfig>>,

    #[account(
        init_if_needed,
        payer = authority,
        space = LiquidityRiskSnapshot::LEN,
        seeds = [
            RISK_SNAPSHOT_SEED,
            market.key().as_ref()
        ],
        bump,
    )]
    pub risk_snapshot: Box<Account<'info, LiquidityRiskSnapshot>>,

    pub system_program: Program<'info, System>,
}

pub fn handle_update_liquidity_risk(
    ctx: Context<UpdateLiquidityRisk>,
    quote_collateral_in: u64,
    quote_usdc_out: u64,
) -> Result<()> {
    require!(quote_collateral_in > 0, LendingError::InvalidAmount);
    require!(quote_usdc_out > 0, LendingError::InvalidAmount);

    // Validate that this quote can produce a valid execution LTV.
    //
    // Importantly, the authority is NOT publishing an LTV.
    // The program derives the LTV from executable output.
    execution_ltv_from_quote(&ctx.accounts.market, quote_usdc_out)?;

    let clock = Clock::get()?;

    let snapshot = &mut ctx.accounts.risk_snapshot;

    snapshot.market = ctx.accounts.market.key();
    snapshot.quote_collateral_in = quote_collateral_in;
    snapshot.quote_usdc_out = quote_usdc_out;
    snapshot.observed_slot = clock.slot;
    snapshot.bump = ctx.bumps.risk_snapshot;

    Ok(())
}
