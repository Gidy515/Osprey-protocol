use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{
    transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked,
};

use crate::{
    constants::*,
    error::LendingError,
    state::{MarketConfig, Position},
};

#[derive(Accounts)]
pub struct DepositCollateral<'info> {
    /// User depositing collateral.
    #[account(mut)]
    pub user: Signer<'info>,

    /// Existing market configuration.
    #[account(
        seeds = [MARKET_SEED, market.collateral_mint.as_ref()],
        bump = market.bump,
    )]
    pub market: Account<'info, MarketConfig>,

    /// One position per user per market.
    #[account(
        init_if_needed,
        payer = user,
        space = Position::LEN,
        seeds = [
            POSITION_SEED,
            market.key().as_ref(),
            user.key().as_ref()
        ],
        bump,
    )]
    pub position: Account<'info, Position>,

    /// The collateral mint configured for this market.
    #[account(
        address = market.collateral_mint @ LendingError::MarketMismatch
    )]
    pub collateral_mint: InterfaceAccount<'info, Mint>,

    /// User's collateral token account.
    #[account(
        mut,
        associated_token::mint = collateral_mint,
        associated_token::authority = user,
        associated_token::token_program = token_program,
    )]
    pub user_collateral_account: InterfaceAccount<'info, TokenAccount>,

    /// PDA authority for the market's collateral vault.
    /// CHECK: PDA is constrained by deterministic seeds.
    #[account(
        seeds = [VAULT_SEED, market.key().as_ref()],
        bump,
    )]
    pub vault_authority: UncheckedAccount<'info>,

    /// Protocol collateral vault.
    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = collateral_mint,
        associated_token::authority = vault_authority,
        associated_token::token_program = token_program,
    )]
    pub vault_collateral_account: InterfaceAccount<'info, TokenAccount>,

    /// Token / Token-2022 program.
    pub token_program: Interface<'info, TokenInterface>,

    /// Associated Token Account program.
    pub associated_token_program: Program<'info, AssociatedToken>,

    /// System program required for account creation.
    pub system_program: Program<'info, System>,
}

pub fn handle_deposit_collateral(ctx: Context<DepositCollateral>, amount: u64) -> Result<()> {
    require!(amount > 0, LendingError::InvalidAmount);

    // ---------------------------------------------------------
    // 1. Initialize the position identity on first deposit.
    // ---------------------------------------------------------

    let position = &mut ctx.accounts.position;

    require!(
        position.owner == Pubkey::default() || position.owner == ctx.accounts.user.key(),
        LendingError::MarketMismatch
    );

    require!(
        position.market == Pubkey::default() || position.market == ctx.accounts.market.key(),
        LendingError::MarketMismatch
    );

    if position.owner == Pubkey::default() {
        position.owner = ctx.accounts.user.key();
        position.market = ctx.accounts.market.key();
        position.bump = ctx.bumps.position;
    }

    require_keys_eq!(
        position.owner,
        ctx.accounts.user.key(),
        LendingError::MarketMismatch
    );

    require_keys_eq!(
        position.market,
        ctx.accounts.market.key(),
        LendingError::MarketMismatch
    );

    // ---------------------------------------------------------
    // 2. Record vault balance before the transfer.
    // ---------------------------------------------------------

    let vault_balance_before = ctx.accounts.vault_collateral_account.amount;

    // ---------------------------------------------------------
    // 3. Transfer collateral from the user to the vault.
    //
    // transfer_checked works with both the legacy SPL Token
    // program and Token-2022 through TokenInterface.
    // ---------------------------------------------------------

    let decimals = ctx.accounts.collateral_mint.decimals;

    let cpi_accounts = TransferChecked {
        from: ctx.accounts.user_collateral_account.to_account_info(),
        mint: ctx.accounts.collateral_mint.to_account_info(),
        to: ctx.accounts.vault_collateral_account.to_account_info(),
        authority: ctx.accounts.user.to_account_info(),
    };

    let cpi_ctx = CpiContext::new(ctx.accounts.token_program.key(), cpi_accounts);

    transfer_checked(cpi_ctx, amount, decimals)?;

    // ---------------------------------------------------------
    // 4. Reload the vault so we know exactly how many tokens
    //    actually arrived.
    //
    // This matters for Token-2022 because the mint can charge
    // a transfer fee.
    // ---------------------------------------------------------

    ctx.accounts.vault_collateral_account.reload()?;

    let vault_balance_after = ctx.accounts.vault_collateral_account.amount;

    let received = vault_balance_after
        .checked_sub(vault_balance_before)
        .ok_or(LendingError::MathOverflow)?;

    require!(received > 0, LendingError::InvalidAmount);

    // ---------------------------------------------------------
    // 5. Record the actual collateral received by the protocol.
    // ---------------------------------------------------------

    position.collateral_amount = position
        .collateral_amount
        .checked_add(received)
        .ok_or(LendingError::MathOverflow)?;

    Ok(())
}
