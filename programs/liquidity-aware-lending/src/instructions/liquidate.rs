use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use crate::{
    constants::*,
    error::LendingError,
    integrations::meteora::{
        event_authority, invoke_swap2, RemainingAccountsInfo, Swap2Accounts,
        METEORA_DLMM_PROGRAM_ID,
    },
    state::{MarketConfig, Position},
};

#[derive(Accounts)]
pub struct Liquidate<'info> {
    /// Account performing the liquidation.
    #[account(mut)]
    pub liquidator: Signer<'info>,

    /// Existing market configuration.
    #[account(
        seeds = [MARKET_SEED, market.collateral_mint.as_ref()],
        bump = market.bump,
        constraint = market.dlmm_pool == lb_pair.key() @ LendingError::MarketMismatch,
    )]
    pub market: Box<Account<'info, MarketConfig>>,

    /// Position being liquidated.
    #[account(
        mut,
        seeds = [
            POSITION_SEED,
            market.key().as_ref(),
            position.owner.as_ref()
        ],
        bump = position.bump,
        constraint = position.market == market.key() @ LendingError::MarketMismatch,
    )]
    pub position: Box<Account<'info, Position>>,

    /// Collateral mint configured for this market.
    #[account(
        address = market.collateral_mint @ LendingError::MarketMismatch
    )]
    pub collateral_mint: Box<InterfaceAccount<'info, Mint>>,

    /// Liquidator's collateral token account.
    #[account(
        mut,
        token::mint = collateral_mint,
        token::authority = liquidator,
        token::token_program = token_2022_program,
    )]
    pub liquidator_collateral_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// PDA controlling the protocol vaults.
    /// CHECK: PDA is constrained by deterministic seeds.
    #[account(
        seeds = [VAULT_SEED, market.key().as_ref()],
        bump,
    )]
    pub vault_authority: UncheckedAccount<'info>,

    /// Protocol collateral vault.
    #[account(
        mut,
        token::mint = collateral_mint,
        token::authority = vault_authority,
        token::token_program = token_2022_program,
    )]
    pub vault_collateral_account: Box<InterfaceAccount<'info, TokenAccount>>,

    /// Debt mint configured for this market.
    #[account(
        address = market.debt_mint @ LendingError::DebtMintMismatch
    )]
    pub debt_mint: Box<InterfaceAccount<'info, Mint>>,

    /// Protocol debt vault.
    #[account(
        mut,
        token::mint = debt_mint,
        token::authority = vault_authority,
        token::token_program = token_program,
    )]
    pub debt_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    // ---------------------------------------------------------
    // Meteora DLMM Swap2 accounts
    // ---------------------------------------------------------
    /// Meteora DLMM pool.
    /// CHECK: Validated against market.dlmm_pool.
    #[account(
        mut,
        address = market.dlmm_pool @ LendingError::MarketMismatch,
        owner = METEORA_DLMM_PROGRAM_ID,
    )]
    pub lb_pair: UncheckedAccount<'info>,

    /// Meteora X reserve.
    ///
    /// For the selected ANTHROPIC-USDC pool, X is ANTHROPIC.
    /// CHECK: Meteora validates the reserve relationship.
    #[account(mut)]
    pub reserve_x: UncheckedAccount<'info>,

    /// Meteora Y reserve.
    ///
    /// For the selected ANTHROPIC-USDC pool, Y is USDC.
    /// CHECK: Meteora validates the reserve relationship.
    #[account(mut)]
    pub reserve_y: UncheckedAccount<'info>,

    /// Meteora oracle.
    /// CHECK: Meteora validates this account.
    #[account(mut)]
    pub oracle: UncheckedAccount<'info>,

    /// Meteora X mint.
    #[account(
        address = market.collateral_mint @ LendingError::MarketMismatch
    )]
    pub token_x_mint: Box<InterfaceAccount<'info, Mint>>,

    /// Meteora Y mint.
    #[account(
        address = market.debt_mint @ LendingError::DebtMintMismatch
    )]
    pub token_y_mint: Box<InterfaceAccount<'info, Mint>>,

    /// Meteora event authority.
    /// CHECK: Deterministically derived from the Meteora program.
    #[account(
        address = event_authority(),
    )]
    pub meteora_event_authority: UncheckedAccount<'info>,

    /// Meteora DLMM program.
    ///
    /// This account is also used as the sentinel value for optional writable
    /// accounts in Meteora's Swap2 account layout.
    /// CHECK: This account is constrained to the Meteora DLMM program ID
    /// and is used as the CPI program account for the Swap2 instruction.
    #[account(mut, address = METEORA_DLMM_PROGRAM_ID)]
    pub meteora_program: UncheckedAccount<'info>,

    /// Memo program required by Meteora Swap2.
    /// CHECK: Fixed Memo program address.
    #[account(
        address = pubkey!("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr"),
    )]
    pub memo_program: UncheckedAccount<'info>,

    pub token_2022_program: Interface<'info, TokenInterface>,

    pub token_program: Interface<'info, TokenInterface>,
}

pub fn handle_liquidate<'info>(
    ctx: Context<'info, Liquidate<'info>>,
    collateral_to_sell: u64,
    min_amount_out: u64,
) -> Result<()> {
    require!(
        collateral_to_sell > 0,
        LendingError::InvalidAmount
    );

    require!(
        min_amount_out > 0,
        LendingError::InvalidAmount
    );

    // -------------------------------------------------------------------------
    // 1. Validate that the position is currently liquidatable.
    // -------------------------------------------------------------------------

    validate_liquidation(
        &ctx.accounts.market,
        &ctx.accounts.position,
        ctx.accounts.collateral_mint.decimals,
    )?;

    // -------------------------------------------------------------------------
    // 2. Record the debt-vault balance BEFORE the Meteora execution.
    //
    // We will use the balance delta after the CPI to determine the actual
    // amount of USDC recovered by the liquidation.
    // -------------------------------------------------------------------------

    let debt_vault_before = ctx.accounts.debt_vault.amount;

    // -------------------------------------------------------------------------
    // 3. Build Meteora remaining-account metadata.
    //
    // For the current MVP, Token-2022 transfer hooks are disabled on the
    // selected PreStocks collateral, so no transfer-hook account slices
    // are required.
    //
    // Remaining accounts are therefore the Meteora bin-array accounts plus
    // any other accounts supplied by the caller.
    // -------------------------------------------------------------------------

    let remaining_accounts = ctx.remaining_accounts;

    let remaining_accounts_info = RemainingAccountsInfo {
        slices: vec![],
    };

    let remaining_account_metas = remaining_accounts
        .iter()
        .map(|account| {
            if account.is_writable {
                anchor_lang::solana_program::instruction::AccountMeta::new(
                    *account.key,
                    account.is_signer,
                )
            } else {
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    *account.key,
                    account.is_signer,
                )
            }
        })
        .collect::<Vec<_>>();

    // -------------------------------------------------------------------------
    // 4. Construct the Meteora Swap2 account set.
    // -------------------------------------------------------------------------

    let swap_accounts = Swap2Accounts {
        lb_pair: ctx.accounts.lb_pair.key(),
        bin_array_bitmap_extension: None,
        reserve_x: ctx.accounts.reserve_x.key(),
        reserve_y: ctx.accounts.reserve_y.key(),
        user_token_in: ctx.accounts.vault_collateral_account.key(),
        user_token_out: ctx.accounts.debt_vault.key(),
        token_x_mint: ctx.accounts.token_x_mint.key(),
        token_y_mint: ctx.accounts.token_y_mint.key(),
        oracle: ctx.accounts.oracle.key(),
        host_fee_in: None,
        user: ctx.accounts.vault_authority.key(),
        token_x_program: ctx.accounts.token_2022_program.key(),
        token_y_program: ctx.accounts.token_program.key(),
        memo_program: ctx.accounts.memo_program.key(),
        event_authority: ctx.accounts.meteora_event_authority.key(),
        program: ctx.accounts.meteora_program.key(),
    };

    // -------------------------------------------------------------------------
    // 5. Build the vault PDA signer.
    //
    // The vault authority controls the collateral vault and debt vault.
    // Meteora therefore receives the vault authority as its swap user.
    // -------------------------------------------------------------------------

    let vault_bump = ctx.bumps.vault_authority;
    let market_key = ctx.accounts.market.key();

    let vault_signer_seeds: &[&[u8]] = &[
        VAULT_SEED,
        market_key.as_ref(),
        &[vault_bump],
    ];

    // -------------------------------------------------------------------------
    // 6. Construct the complete account-info list expected by Meteora.
    // -------------------------------------------------------------------------

    let mut account_infos = vec![
        ctx.accounts.lb_pair.to_account_info(),

        // Bitmap-extension sentinel.
        ctx.accounts.meteora_program.to_account_info(),

        ctx.accounts.reserve_x.to_account_info(),
        ctx.accounts.reserve_y.to_account_info(),

        ctx.accounts.vault_collateral_account.to_account_info(),
        ctx.accounts.debt_vault.to_account_info(),

        ctx.accounts.token_x_mint.to_account_info(),
        ctx.accounts.token_y_mint.to_account_info(),

        ctx.accounts.oracle.to_account_info(),

        // Host-fee sentinel.
        ctx.accounts.meteora_program.to_account_info(),

        ctx.accounts.vault_authority.to_account_info(),

        ctx.accounts.token_2022_program.to_account_info(),
        ctx.accounts.token_program.to_account_info(),

        ctx.accounts.memo_program.to_account_info(),
        ctx.accounts.meteora_event_authority.to_account_info(),

        // Meteora program.
        ctx.accounts.meteora_program.to_account_info(),
    ];

    account_infos.extend(
        remaining_accounts.iter().cloned()
    );

    // -------------------------------------------------------------------------
    // 7. Execute the actual Meteora swap.
    //
    // collateral_to_sell is sent from the protocol collateral vault.
    // The resulting USDC is sent to the protocol debt vault.
    // -------------------------------------------------------------------------

    invoke_swap2(
        swap_accounts,
        collateral_to_sell,
        min_amount_out,
        remaining_account_metas,
        remaining_accounts_info,
        &account_infos,
        vault_signer_seeds,
    )?;

    // -------------------------------------------------------------------------
    // 8. Reload the debt vault.
    //
    // The Token-2022 CPI modified the account data, so reload is required
    // before reading the updated token balance.
    // -------------------------------------------------------------------------

    ctx.accounts.debt_vault.reload()?;

    let debt_vault_after = ctx.accounts.debt_vault.amount;

    // -------------------------------------------------------------------------
    // 9. Calculate the actual USDC recovered by the liquidation.
    //
    // We use the balance delta rather than trusting a caller-provided amount.
    // -------------------------------------------------------------------------

    let usdc_recovered = debt_vault_after
        .checked_sub(debt_vault_before)
        .ok_or(LendingError::MathOverflow)?;

    require!(
        usdc_recovered > 0,
        LendingError::InsufficientDebtLiquidity
    );

    // -------------------------------------------------------------------------
    // 10. Validate that the actual execution was sufficient to restore the
    // position to a healthy state.
    //
    // This checks:
    //
    // - collateral_to_sell <= configured liquidation cap
    // - actual USDC recovered >= required repayment
    // - liquidation can restore the position to the maximum LTV
    // -------------------------------------------------------------------------

    let debt_repaid = validate_liquidation_execution(
        ctx.accounts.position.collateral_amount,
        ctx.accounts.position.debt_amount,
        collateral_to_sell,
        usdc_recovered,
        ctx.accounts.market.collateral_price_usdc,
        ctx.accounts.collateral_mint.decimals,
        ctx.accounts.market.max_ltv_bps,
        ctx.accounts.market.max_liquidation_bps,
    )?;

    // -------------------------------------------------------------------------
    // 11. Update position accounting.
    //
    // The collateral was consumed by the Meteora swap, so remove it from
    // the position.
    //
    // The actual USDC recovered reduces the outstanding debt, capped at the
    // position's remaining debt.
    // -------------------------------------------------------------------------

    ctx.accounts.position.collateral_amount = ctx
        .accounts
        .position
        .collateral_amount
        .checked_sub(collateral_to_sell)
        .ok_or(LendingError::MathOverflow)?;

    ctx.accounts.position.debt_amount = ctx
        .accounts
        .position
        .debt_amount
        .checked_sub(debt_repaid)
        .ok_or(LendingError::MathOverflow)?;

    Ok(())
}

/// Returns the maximum debt allowed against a given collateral value.
///
/// Formula:
///
/// max_debt = collateral_value * LTV / 10_000
pub fn max_debt_for_collateral(collateral_value_usdc: u64, ltv_bps: u16) -> Result<u64> {
    collateral_value_usdc
        .checked_mul(ltv_bps as u64)
        .ok_or_else(|| error!(LendingError::MathOverflow))?
        .checked_div(10_000)
        .ok_or_else(|| error!(LendingError::MathOverflow))
}

/// Returns whether a position is liquidatable.
///
/// A position becomes liquidatable when:
///
/// debt > collateral_value * liquidation_threshold
pub fn is_liquidatable(
    collateral_value_usdc: u64,
    debt_amount: u64,
    liquidation_threshold_bps: u16,
) -> Result<bool> {
    let liquidation_limit =
        max_debt_for_collateral(collateral_value_usdc, liquidation_threshold_bps)?;

    Ok(debt_amount > liquidation_limit)
}

/// Returns the maximum amount of collateral that can be liquidated
/// according to the configured liquidation cap.
///
/// Example:
///
/// collateral = 1,000 tokens
/// max_liquidation_bps = 2,500
///
/// maximum liquidation = 250 tokens
pub fn max_liquidation_amount(collateral_amount: u64, max_liquidation_bps: u16) -> Result<u64> {
    collateral_amount
        .checked_mul(max_liquidation_bps as u64)
        .ok_or_else(|| error!(LendingError::MathOverflow))?
        .checked_div(10_000)
        .ok_or_else(|| error!(LendingError::MathOverflow))
}

/// Returns the collateral value in USDC base units.
///
/// collateral_amount is expressed in the collateral mint's base units.
///
/// collateral_price_usdc is expressed in USDC base units per whole
/// collateral token.
pub fn collateral_value_usdc(
    collateral_amount: u64,
    collateral_price_usdc: u64,
    collateral_decimals: u8,
) -> Result<u64> {
    collateral_amount
        .checked_mul(collateral_price_usdc)
        .ok_or_else(|| error!(LendingError::MathOverflow))?
        .checked_div(10u64.pow(collateral_decimals as u32))
        .ok_or_else(|| error!(LendingError::MathOverflow))
}

/// Calculates the collateral amount required to cover a debt amount
/// while accounting for the liquidation bonus.
///
/// The liquidator must receive:
///
/// debt + liquidation bonus
///
/// worth of collateral.
pub fn collateral_required_for_debt(
    debt_amount: u64,
    collateral_price_usdc: u64,
    collateral_decimals: u8,
    liquidation_bonus_bps: u16,
) -> Result<u64> {
    let debt_with_bonus = debt_amount
        .checked_mul(10_000u64 + liquidation_bonus_bps as u64)
        .ok_or_else(|| error!(LendingError::MathOverflow))?
        .checked_div(10_000)
        .ok_or_else(|| error!(LendingError::MathOverflow))?;

    debt_with_bonus
        .checked_mul(10u64.pow(collateral_decimals as u32))
        .ok_or_else(|| error!(LendingError::MathOverflow))?
        .checked_div(collateral_price_usdc)
        .ok_or_else(|| error!(LendingError::MathOverflow))
}

/// Returns the maximum debt that can remain after liquidating
/// a given amount of collateral.
///
/// This is useful when checking whether a partial liquidation
/// actually restores the position to a healthy state.
pub fn remaining_debt_capacity(
    remaining_collateral: u64,
    collateral_price_usdc: u64,
    collateral_decimals: u8,
    max_ltv_bps: u16,
) -> Result<u64> {
    let remaining_value = collateral_value_usdc(
        remaining_collateral,
        collateral_price_usdc,
        collateral_decimals,
    )?;

    max_debt_for_collateral(remaining_value, max_ltv_bps)
}

/// Calculates the minimum USDC repayment required to restore a
/// position to the maximum allowed LTV.
///
/// Formula:
///
/// required_repayment =
///     debt - (remaining_collateral_value × max_ltv)
///
/// If the result is zero, the position is already healthy.
///
/// `usdc_recovered` is deliberately NOT used here because this
/// function determines the debt reduction required. The caller
/// later checks whether the actual Meteora execution produced
/// enough USDC to satisfy this requirement.
pub fn required_debt_repayment(
    collateral_amount: u64,
    debt_amount: u64,
    collateral_to_sell: u64,
    collateral_price_usdc: u64,
    collateral_decimals: u8,
    max_ltv_bps: u16,
) -> Result<u64> {
    require!(
        collateral_to_sell <= collateral_amount,
        LendingError::InsufficientCollateral
    );

    let remaining_collateral = collateral_amount
        .checked_sub(collateral_to_sell)
        .ok_or(LendingError::MathOverflow)?;

    let remaining_capacity = remaining_debt_capacity(
        remaining_collateral,
        collateral_price_usdc,
        collateral_decimals,
        max_ltv_bps,
    )?;

    if debt_amount <= remaining_capacity {
        return Ok(0);
    }

    debt_amount
        .checked_sub(remaining_capacity)
        .ok_or(LendingError::MathOverflow.into())
}

/// Calculates the minimum collateral amount that must be sold
/// when the liquidation execution is assumed to produce a known
/// USDC amount for each unit of collateral sold.
///
/// `usdc_out_per_collateral_unit` is expressed as:
///
/// USDC base units / collateral base unit.
///
/// This is a quote/rate abstraction. The actual Meteora execution
/// will later enforce the quote through `min_amount_out`.
/// Calculates the minimum collateral amount to sell using a quoted
/// collateral-input / USDC-output pair.
///
/// This is a pure math abstraction for testing liquidation logic.
///
/// `quote_collateral_in` and `quote_usdc_out` represent an actual
/// execution quote from the liquidation venue:
///
///     quote_collateral_in -> quote_usdc_out
///
/// The function derives an effective execution ratio from that quote
/// and uses it to estimate USDC recovery for candidate liquidation
/// amounts.
///
/// IMPORTANT:
/// This is NOT the Meteora quote itself. The production liquidation
/// instruction must obtain a fresh Meteora quote/execution constraint
/// and pass the resulting amounts into the liquidation logic.
pub fn minimum_collateral_to_sell(
    collateral_amount: u64,
    debt_amount: u64,
    collateral_price_usdc: u64,
    collateral_decimals: u8,
    max_ltv_bps: u16,
    max_liquidation_bps: u16,
    quote_collateral_in: u64,
    quote_usdc_out: u64,
) -> Result<u64> {
    require!(quote_collateral_in > 0, LendingError::InvalidAmount);

    require!(quote_usdc_out > 0, LendingError::InvalidAmount);

    let max_liquidation = max_liquidation_amount(collateral_amount, max_liquidation_bps)?;

    if max_liquidation == 0 {
        return Err(error!(LendingError::InsufficientCollateral));
    }

    // Binary search for the smallest collateral amount that restores
    // the position to the configured maximum LTV under the supplied
    // execution quote.
    let mut low = 1u64;
    let mut high = max_liquidation;
    let mut answer = None;

    while low <= high {
        let mid = low
            .checked_add(high.checked_sub(low).ok_or(LendingError::MathOverflow)? / 2)
            .ok_or(LendingError::MathOverflow)?;

        // Estimate USDC recovered using the supplied quote:
        //
        // mid / quote_collateral_in
        //     * quote_usdc_out
        //
        // All values remain integer base units.
        let usdc_recovered = mid
            .checked_mul(quote_usdc_out)
            .ok_or(LendingError::MathOverflow)?
            .checked_div(quote_collateral_in)
            .ok_or(LendingError::MathOverflow)?;

        let remaining_collateral = collateral_amount
            .checked_sub(mid)
            .ok_or(LendingError::MathOverflow)?;

        let remaining_capacity = remaining_debt_capacity(
            remaining_collateral,
            collateral_price_usdc,
            collateral_decimals,
            max_ltv_bps,
        )?;

        let remaining_debt = debt_amount.saturating_sub(usdc_recovered);

        if remaining_debt <= remaining_capacity {
            answer = Some(mid);

            high = mid.checked_sub(1).ok_or(LendingError::MathOverflow)?;
        } else {
            low = mid.checked_add(1).ok_or(LendingError::MathOverflow)?;
        }
    }

    answer.ok_or_else(|| error!(LendingError::LiquidationCannotRestoreHealth))
}

/// Validates the result of a liquidation swap.
///
/// `collateral_to_sell` is the amount of collateral that was actually sent
/// to the liquidation venue.
///
/// `usdc_recovered` is the actual USDC received by the protocol vault after
/// the swap. It must come from the vault balance delta in the production
/// instruction, not from an untrusted caller argument.
///
/// The liquidation is successful only if the recovered USDC is sufficient
/// to bring the remaining debt back within the maximum LTV.
pub fn validate_liquidation_execution(
    collateral_amount: u64,
    debt_amount: u64,
    collateral_to_sell: u64,
    usdc_recovered: u64,
    collateral_price_usdc: u64,
    collateral_decimals: u8,
    max_ltv_bps: u16,
    max_liquidation_bps: u16,
) -> Result<u64> {
    require!(collateral_amount > 0, LendingError::InsufficientCollateral);

    require!(debt_amount > 0, LendingError::InvalidAmount);

    require!(collateral_to_sell > 0, LendingError::InvalidAmount);

    let max_liquidation = max_liquidation_amount(collateral_amount, max_liquidation_bps)?;

    require!(
        collateral_to_sell <= max_liquidation,
        LendingError::LiquidationExceedsCap
    );

    let required_repayment = required_debt_repayment(
        collateral_amount,
        debt_amount,
        collateral_to_sell,
        collateral_price_usdc,
        collateral_decimals,
        max_ltv_bps,
    )?;

    require!(
        usdc_recovered >= required_repayment,
        LendingError::LiquidationCannotRestoreHealth
    );

    Ok(usdc_recovered.min(debt_amount))
}

/// Performs the pure liquidation validation.
///
/// This does NOT execute a swap yet.
///
/// The actual Meteora execution will be added after these
/// invariants are tested.
pub fn validate_liquidation(
    market: &MarketConfig,
    position: &Position,
    collateral_decimals: u8,
) -> Result<()> {
    require!(
        position.collateral_amount > 0,
        LendingError::InsufficientCollateral
    );

    require!(position.debt_amount > 0, LendingError::InvalidAmount);

    let collateral_value = collateral_value_usdc(
        position.collateral_amount,
        market.collateral_price_usdc,
        collateral_decimals,
    )?;

    require!(
        is_liquidatable(
            collateral_value,
            position.debt_amount,
            market.liquidation_threshold_bps,
        )?,
        LendingError::PositionHealthy
    );

    Ok(())
}
