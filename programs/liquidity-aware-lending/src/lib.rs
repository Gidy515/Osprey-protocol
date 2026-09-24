pub mod constants;
pub mod error;
pub mod instructions;
pub mod integrations;
pub mod risk_engine;
pub mod state;
use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("jAUJs14M4WiAvrgRgZiKQ2nunkWMrqqM7sGZotmWwss");

#[program]
pub mod liquidity_aware_lending {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, params: InitializeMarketParams) -> Result<()> {
        instructions::initialize::handle_initialize(ctx, params)
    }

    pub fn update_liquidity_risk(
        ctx: Context<UpdateLiquidityRisk>,
        quote_collateral_in: u64,
        quote_usdc_out: u64,
    ) -> Result<()> {
        instructions::update_liquidity_risk::handle_update_liquidity_risk(
            ctx,
            quote_collateral_in,
            quote_usdc_out,
        )
    }

    pub fn deposit_collateral(ctx: Context<DepositCollateral>, amount: u64) -> Result<()> {
        instructions::deposit_collateral::handle_deposit_collateral(ctx, amount)
    }

    pub fn borrow(ctx: Context<Borrow>, amount: u64) -> Result<()> {
        instructions::borrow::handle_borrow(ctx, amount)
    }

    pub fn repay(ctx: Context<Repay>, amount: u64) -> Result<()> {
        instructions::handle_repay(ctx, amount)
    }

    pub fn withdraw_collateral(ctx: Context<WithdrawCollateral>, amount: u64) -> Result<()> {
        instructions::handle_withdraw_collateral(ctx, amount)
    }

    pub fn liquidate<'info>(
        ctx: Context<'info, Liquidate<'info>>,
        collateral_to_sell: u64,
        min_amount_out: u64,
    ) -> Result<()> {
        instructions::liquidate::handle_liquidate(ctx, collateral_to_sell, min_amount_out)
    }
}
