use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked,
};

use crate::{
    constants::*,
    error::LendingError,
    risk_engine::assess_risk,
    state::{LiquidityRiskSnapshot, MarketConfig, Position},
};

#[derive(Accounts)]
pub struct Borrow<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        seeds = [MARKET_SEED, market.collateral_mint.as_ref()],
        bump = market.bump,
    )]
    pub market: Box<Account<'info, MarketConfig>>,

    #[account(
        mut,
        seeds = [
            POSITION_SEED,
            market.key().as_ref(),
            user.key().as_ref()
        ],
        bump = position.bump,
        constraint = position.owner == user.key() @ LendingError::MarketMismatch,
        constraint = position.market == market.key() @ LendingError::MarketMismatch,
    )]
    pub position: Box<Account<'info, Position>>,

    #[account(
        seeds = [
            RISK_SNAPSHOT_SEED,
            market.key().as_ref()
        ],
        bump = risk_snapshot.bump,
        constraint = risk_snapshot.market == market.key()
            @ LendingError::MarketMismatch,
    )]
    pub risk_snapshot: Box<Account<'info, LiquidityRiskSnapshot>>,

    #[account(
        address = market.collateral_mint @ LendingError::MarketMismatch
    )]
    pub collateral_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        address = market.debt_mint @ LendingError::DebtMintMismatch
    )]
    pub debt_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        mut,
        token::mint = debt_mint,
        token::authority = user,
        token::token_program = token_program,
    )]
    pub user_debt_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK: PDA is constrained by deterministic seeds.
    #[account(
        seeds = [VAULT_SEED, market.key().as_ref()],
        bump,
    )]
    pub vault_authority: UncheckedAccount<'info>,

    #[account(
        mut,
        token::mint = debt_mint,
        token::authority = vault_authority,
        token::token_program = token_program,
    )]
    pub debt_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    pub token_program: Interface<'info, TokenInterface>,
}

pub fn handle_borrow(ctx: Context<Borrow>, amount: u64) -> Result<()> {
    require!(amount > 0, LendingError::InvalidAmount);

    let position = &ctx.accounts.position;
    let market = &ctx.accounts.market;

    let collateral_decimals = ctx.accounts.collateral_mint.decimals;

    let collateral_scale = 10u128
        .checked_pow(collateral_decimals as u32)
        .ok_or(LendingError::MathOverflow)?;

    let collateral_value_usdc_u128 = (position.collateral_amount as u128)
        .checked_mul(market.collateral_price_usdc as u128)
        .ok_or(LendingError::MathOverflow)?
        .checked_div(collateral_scale)
        .ok_or(LendingError::MathOverflow)?;

    let collateral_value_usdc =
        u64::try_from(collateral_value_usdc_u128)
            .map_err(|_| LendingError::MathOverflow)?;

    require!(
        collateral_value_usdc > 0,
        LendingError::InsufficientCollateral
    );

    // ---------------------------------------------------------
    // LIQUIDITY-AWARE RISK ASSESSMENT
    //
    // We no longer use market.max_ltv_bps directly.
    //
    // Meteora executable liquidity
    //          ↓
    // execution-derived LTV
    //          ↓
    // issuer-risk ceiling
    //          ↓
    // effective LTV
    //          ↓
    // max debt
    // ---------------------------------------------------------

    let clock = Clock::get()?;

    let risk = assess_risk(
        market,
        &ctx.accounts.risk_snapshot,
        collateral_value_usdc,
        clock.slot,
    )?;

    let new_debt = position
        .debt_amount
        .checked_add(amount)
        .ok_or(LendingError::MathOverflow)?;

    require!(new_debt <= risk.max_debt, LendingError::BorrowExceedsLtv);

    // Ensure the protocol actually has enough USDC to lend.
    require!(
        ctx.accounts.debt_vault.amount >= amount,
        LendingError::InsufficientDebtLiquidity
    );

    let decimals = ctx.accounts.debt_mint.decimals;

    let cpi_accounts = TransferChecked {
        from: ctx.accounts.debt_vault.to_account_info(),
        mint: ctx.accounts.debt_mint.to_account_info(),
        to: ctx.accounts.user_debt_account.to_account_info(),
        authority: ctx.accounts.vault_authority.to_account_info(),
    };

    let market_key = market.key();
    let bump = [ctx.bumps.vault_authority];

    let signer_seeds: &[&[u8]] = &[VAULT_SEED, market_key.as_ref(), &bump];

    let signer = [signer_seeds];

    let cpi_ctx =
        CpiContext::new(ctx.accounts.token_program.key(), cpi_accounts).with_signer(&signer);

    transfer_checked(cpi_ctx, amount, decimals)?;

    let position = &mut ctx.accounts.position;

    position.debt_amount = new_debt;

    Ok(())
}
