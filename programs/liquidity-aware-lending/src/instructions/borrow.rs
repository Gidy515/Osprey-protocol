use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked,
};

use crate::{
    constants::*,
    error::LendingError,
    state::{MarketConfig, Position},
};

#[derive(Accounts)]
pub struct Borrow<'info> {
    /// User borrowing USDC against their collateral.
    #[account(mut)]
    pub user: Signer<'info>,

    /// Existing market configuration.
    #[account(
        seeds = [MARKET_SEED, market.collateral_mint.as_ref()],
        bump = market.bump,
    )]
    pub market: Box<Account<'info, MarketConfig>>,

    /// User's borrowing position.
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

    /// Market collateral mint.
    #[account(
        address = market.collateral_mint @ LendingError::MarketMismatch
    )]
    pub collateral_mint: Box<InterfaceAccount<'info, Mint>>,

    /// Market debt mint.
    #[account(
        address = market.debt_mint @ LendingError::DebtMintMismatch
    )]
    pub debt_mint: Box<InterfaceAccount<'info, Mint>>,

    /// User's USDC account.
    #[account(
        mut,
        token::mint = debt_mint,
        token::authority = user,
        token::token_program = token_program,
    )]
    pub user_debt_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// PDA authority for the market's token vaults.
    /// CHECK: PDA is constrained by deterministic seeds.
    #[account(
        seeds = [VAULT_SEED, market.key().as_ref()],
        bump,
    )]
    pub vault_authority: UncheckedAccount<'info>,

    /// Protocol USDC liquidity vault.
    #[account(
        mut,
        token::mint = debt_mint,
        token::authority = vault_authority,
        token::token_program = token_program,
    )]
    pub debt_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Token / Token-2022 program.
    pub token_program: Interface<'info, TokenInterface>,
}

pub fn handle_borrow(ctx: Context<Borrow>, amount: u64) -> Result<()> {
    require!(amount > 0, LendingError::InvalidAmount);

    let position = &ctx.accounts.position;
    let market = &ctx.accounts.market;

    // ---------------------------------------------------------
    // 1. Convert collateral amount into USDC value.
    //
    // collateral_amount uses the collateral mint's decimals.
    // collateral_price_usdc represents the USDC base-unit
    // value of one whole collateral token.
    // ---------------------------------------------------------

    let collateral_decimals = ctx.accounts.collateral_mint.decimals;

    let collateral_value_usdc = position
        .collateral_amount
        .checked_mul(market.collateral_price_usdc)
        .ok_or(LendingError::MathOverflow)?
        .checked_div(10u64.pow(collateral_decimals as u32))
        .ok_or(LendingError::MathOverflow)?;

    require!(
        collateral_value_usdc > 0,
        LendingError::InsufficientCollateral
    );

    // ---------------------------------------------------------
    // 2. Calculate the maximum debt permitted by max LTV.
    // ---------------------------------------------------------

    let max_debt = collateral_value_usdc
        .checked_mul(market.max_ltv_bps as u64)
        .ok_or(LendingError::MathOverflow)?
        .checked_div(10_000)
        .ok_or(LendingError::MathOverflow)?;

    // ---------------------------------------------------------
    // 3. Add the requested borrow to existing debt.
    // ---------------------------------------------------------

    let new_debt = position
        .debt_amount
        .checked_add(amount)
        .ok_or(LendingError::MathOverflow)?;

    require!(new_debt <= max_debt, LendingError::BorrowExceedsLtv);

    // ---------------------------------------------------------
    // 4. Transfer USDC from the protocol liquidity vault
    //    to the borrower.
    // ---------------------------------------------------------

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

    // ---------------------------------------------------------
    // 5. Record the new debt only after the transfer succeeds.
    // ---------------------------------------------------------

    let position = &mut ctx.accounts.position;

    position.debt_amount = new_debt;

    Ok(())
}
