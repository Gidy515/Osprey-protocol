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
pub struct WithdrawCollateral<'info> {
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
        address = market.collateral_mint @ LendingError::MarketMismatch
    )]
    pub collateral_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        mut,
        token::mint = collateral_mint,
        token::authority = user,
        token::token_program = token_program,
    )]
    pub user_collateral_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK: PDA is constrained by deterministic seeds.
    #[account(
        seeds = [VAULT_SEED, market.key().as_ref()],
        bump,
    )]
    pub vault_authority: UncheckedAccount<'info>,

    #[account(
        mut,
        token::mint = collateral_mint,
        token::authority = vault_authority,
        token::token_program = token_program,
    )]
    pub vault_collateral_account: Box<InterfaceAccount<'info, TokenAccount>>,

    pub token_program: Interface<'info, TokenInterface>,
}

pub fn handle_withdraw_collateral(ctx: Context<WithdrawCollateral>, amount: u64) -> Result<()> {
    require!(amount > 0, LendingError::InvalidAmount);

    let position = &ctx.accounts.position;
    let market = &ctx.accounts.market;

    require!(
        amount <= position.collateral_amount,
        LendingError::InsufficientCollateral
    );

    let remaining_collateral = position
        .collateral_amount
        .checked_sub(amount)
        .ok_or(LendingError::MathOverflow)?;

    let collateral_decimals = ctx.accounts.collateral_mint.decimals;

    let remaining_collateral_value_usdc = remaining_collateral
        .checked_mul(market.collateral_price_usdc)
        .ok_or(LendingError::MathOverflow)?
        .checked_div(10u64.pow(collateral_decimals as u32))
        .ok_or(LendingError::MathOverflow)?;

    if position.debt_amount > 0 {
        let max_debt = remaining_collateral_value_usdc
            .checked_mul(market.max_ltv_bps as u64)
            .ok_or(LendingError::MathOverflow)?
            .checked_div(10_000)
            .ok_or(LendingError::MathOverflow)?;

        require!(
            position.debt_amount <= max_debt,
            LendingError::WithdrawalViolatesLtv
        );
    }

    let decimals = ctx.accounts.collateral_mint.decimals;

    let cpi_accounts = TransferChecked {
        from: ctx.accounts.vault_collateral_account.to_account_info(),
        mint: ctx.accounts.collateral_mint.to_account_info(),
        to: ctx.accounts.user_collateral_account.to_account_info(),
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
    position.collateral_amount = remaining_collateral;

    Ok(())
}
