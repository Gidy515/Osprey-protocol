use anchor_lang::prelude::*;

use crate::{constants::*, error::LendingError, state::MarketConfig};

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct InitializeMarketParams {
    pub collateral_mint: Pubkey,
    pub debt_mint: Pubkey,
    pub dlmm_pool: Pubkey,
    pub collateral_price_usdc: u64,
}

#[derive(Accounts)]
#[instruction(params: InitializeMarketParams)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = authority,
        space = MarketConfig::LEN,
        seeds = [MARKET_SEED, params.collateral_mint.as_ref()],
        bump
    )]
    pub market: Account<'info, MarketConfig>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handle_initialize(ctx: Context<Initialize>, params: InitializeMarketParams) -> Result<()> {
    require!(
        params.collateral_mint != Pubkey::default(),
        LendingError::InvalidMarketConfig
    );

    require!(
        params.debt_mint != Pubkey::default(),
        LendingError::InvalidMarketConfig
    );

    require!(
        params.dlmm_pool != Pubkey::default(),
        LendingError::InvalidMarketConfig
    );

    require!(
        params.collateral_price_usdc > 0,
        LendingError::InvalidMarketConfig
    );

    require!(
        MIN_LTV_BPS < MAX_LTV_BPS,
        LendingError::InvalidLtvConfiguration
    );

    require!(
        MAX_LTV_BPS < LIQUIDATION_THRESHOLD_BPS,
        LendingError::InvalidLtvConfiguration
    );

    require!(
        LIQUIDATION_BONUS_BPS < 10_000,
        LendingError::InvalidLtvConfiguration
    );

    require!(
        MAX_LIQUIDATION_BPS > 0 && MAX_LIQUIDATION_BPS <= 10_000,
        LendingError::InvalidLtvConfiguration
    );

    require!(
        ISSUER_RISK_CEILING_BPS <= MAX_LTV_BPS,
        LendingError::InvalidLtvConfiguration
    );

    require!(
        PRESTOCKS_TRANSFER_FEE_BPS < 10_000,
        LendingError::InvalidMarketConfig
    );

    let market = &mut ctx.accounts.market;

    market.collateral_mint = params.collateral_mint;
    market.debt_mint = params.debt_mint;
    market.collateral_price_usdc = params.collateral_price_usdc;
    market.dlmm_pool = params.dlmm_pool;
    market.authority = ctx.accounts.authority.key();

    market.max_ltv_bps = MAX_LTV_BPS;
    market.min_ltv_bps = MIN_LTV_BPS;
    market.liquidation_threshold_bps = LIQUIDATION_THRESHOLD_BPS;
    market.liquidation_bonus_bps = LIQUIDATION_BONUS_BPS;
    market.max_liquidation_bps = MAX_LIQUIDATION_BPS;
    market.reference_liquidation_size_usdc = REFERENCE_LIQUIDATION_SIZE_USDC;
    market.issuer_risk_ceiling_bps = ISSUER_RISK_CEILING_BPS;
    market.transfer_fee_bps = PRESTOCKS_TRANSFER_FEE_BPS;
    market.bump = ctx.bumps.market;

    Ok(())
}
