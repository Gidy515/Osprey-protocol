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
pub struct Repay<'info> {
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

pub fn handle_repay(ctx: Context<Repay>, amount: u64) -> Result<()> {
    require!(amount > 0, LendingError::InvalidAmount);

    let current_debt = ctx.accounts.position.debt_amount;

    require!(amount <= current_debt, LendingError::InvalidAmount);

    let decimals = ctx.accounts.debt_mint.decimals;

    let cpi_accounts = TransferChecked {
        from: ctx.accounts.user_debt_account.to_account_info(),
        mint: ctx.accounts.debt_mint.to_account_info(),
        to: ctx.accounts.debt_vault.to_account_info(),
        authority: ctx.accounts.user.to_account_info(),
    };

    let cpi_ctx = CpiContext::new(ctx.accounts.token_program.key(), cpi_accounts);

    transfer_checked(cpi_ctx, amount, decimals)?;

    let position = &mut ctx.accounts.position;
    position.debt_amount = current_debt
        .checked_sub(amount)
        .ok_or(LendingError::MathOverflow)?;

    Ok(())
}
