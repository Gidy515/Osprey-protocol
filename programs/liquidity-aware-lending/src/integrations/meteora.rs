use anchor_lang::{
    prelude::*,
    solana_program::{
        account_info::AccountInfo,
        instruction::{AccountMeta, Instruction},
        program::invoke_signed,
    },
    AnchorDeserialize, AnchorSerialize,
};

use crate::error::LendingError;

/// Meteora DLMM / lb_clmm program ID.
///
/// This is the address declared by the current Meteora DLMM IDL.
pub const METEORA_DLMM_PROGRAM_ID: Pubkey = pubkey!("LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo");

/// Meteora's event-authority PDA seed.
pub const METEORA_EVENT_AUTHORITY_SEED: &[u8] = b"__event_authority";

/// Derives Meteora's event-authority PDA.
pub fn event_authority() -> Pubkey {
    Pubkey::find_program_address(&[METEORA_EVENT_AUTHORITY_SEED], &METEORA_DLMM_PROGRAM_ID).0
}

/// Exact AccountsType definition from the current Meteora DLMM IDL.
///
/// These variants describe groups of remaining accounts used by
/// Token-2022 transfer hooks and related flows.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq, Eq)]
pub enum AccountsType {
    TransferHookX,
    TransferHookY,
    TransferHookReward,
    TransferHookMultiReward(u8),
    TransferHookReferral,
}

/// Exact RemainingAccountsSlice definition from the current
/// Meteora DLMM IDL.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct RemainingAccountsSlice {
    pub accounts_type: AccountsType,
    pub length: u8,
}

/// Exact RemainingAccountsInfo definition from the current
/// Meteora DLMM IDL.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct RemainingAccountsInfo {
    pub slices: Vec<RemainingAccountsSlice>,
}

/// Fixed Swap2 accounts.
///
/// This mirrors the current Meteora DLMM Swap2 account layout.
///
/// Optional Meteora accounts are represented as Option<Pubkey>.
/// When None, the helper inserts the DLMM program ID as the
/// sentinel used by Meteora's Swap2 account layout.
#[derive(Clone, Debug)]
pub struct Swap2Accounts {
    pub lb_pair: Pubkey,
    pub bin_array_bitmap_extension: Option<Pubkey>,
    pub reserve_x: Pubkey,
    pub reserve_y: Pubkey,
    pub user_token_in: Pubkey,
    pub user_token_out: Pubkey,
    pub token_x_mint: Pubkey,
    pub token_y_mint: Pubkey,
    pub oracle: Pubkey,
    pub host_fee_in: Option<Pubkey>,
    pub user: Pubkey,
    pub token_x_program: Pubkey,
    pub token_y_program: Pubkey,
    pub memo_program: Pubkey,
    pub event_authority: Pubkey,
    pub program: Pubkey,
}

/// Exact Swap2 instruction arguments from the current Meteora IDL.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct Swap2Args {
    pub amount_in: u64,
    pub min_amount_out: u64,
    pub remaining_accounts_info: RemainingAccountsInfo,
}

/// Serializes the current Meteora Swap2 instruction data.
///
/// Meteora is an external Anchor program, so its discriminator cannot
/// be obtained through this program's `InstructionData` trait.
pub fn serialize_swap2_args(args: &Swap2Args) -> Vec<u8> {
    let mut data = Vec::new();

    // Meteora DLMM Swap2 discriminator:
    // sha256("global:swap2")[0..8]
    data.extend_from_slice(&[65, 75, 63, 76, 235, 91, 91, 136]);

    args.serialize(&mut data)
        .expect("Swap2 serialization should not fail");

    data
}

/// Constructs the exact Meteora Swap2 instruction.
///
/// This function only constructs the instruction.
/// It does not execute a CPI.
///
/// `remaining_accounts` should contain the bin-array accounts and,
/// when applicable, Token-2022 transfer-hook accounts.
pub fn build_swap2_instruction(
    accounts: Swap2Accounts,
    amount_in: u64,
    min_amount_out: u64,
    remaining_accounts: Vec<AccountMeta>,
    remaining_accounts_info: RemainingAccountsInfo,
) -> Result<Instruction> {
    require!(
        accounts.program == METEORA_DLMM_PROGRAM_ID,
        LendingError::MarketMismatch
    );

    require!(amount_in > 0, LendingError::InvalidAmount);

    require!(min_amount_out > 0, LendingError::InvalidAmount);

    let mut account_metas = Vec::with_capacity(16 + remaining_accounts.len());

    // 0. lb_pair
    account_metas.push(AccountMeta::new(accounts.lb_pair, false));

    // 1. bin_array_bitmap_extension (optional)
    let bitmap_extension = accounts
        .bin_array_bitmap_extension
        .unwrap_or(METEORA_DLMM_PROGRAM_ID);

    account_metas.push(AccountMeta::new(bitmap_extension, false));

    // 2. reserve_x
    account_metas.push(AccountMeta::new(accounts.reserve_x, false));

    // 3. reserve_y
    account_metas.push(AccountMeta::new(accounts.reserve_y, false));

    // 4. user_token_in
    account_metas.push(AccountMeta::new(accounts.user_token_in, false));

    // 5. user_token_out
    account_metas.push(AccountMeta::new(accounts.user_token_out, false));

    // 6. token_x_mint
    account_metas.push(AccountMeta::new_readonly(accounts.token_x_mint, false));

    // 7. token_y_mint
    account_metas.push(AccountMeta::new_readonly(accounts.token_y_mint, false));

    // 8. oracle
    account_metas.push(AccountMeta::new(accounts.oracle, false));

    // 9. host_fee_in
    //
    // Meteora keeps this account position even when no host-fee account
    // is provided. The SDK uses the DLMM program ID as the sentinel.
    let host_fee_in = accounts.host_fee_in.unwrap_or(METEORA_DLMM_PROGRAM_ID);
    account_metas.push(AccountMeta::new(host_fee_in, false));

    // 10. user
    //
    // Meteora requires this account to be a signer.
    account_metas.push(AccountMeta::new_readonly(accounts.user, true));

    // 11. token_x_program
    account_metas.push(AccountMeta::new_readonly(accounts.token_x_program, false));

    // 12. token_y_program
    account_metas.push(AccountMeta::new_readonly(accounts.token_y_program, false));

    // 13. memo_program
    account_metas.push(AccountMeta::new_readonly(accounts.memo_program, false));

    // 14. event_authority
    account_metas.push(AccountMeta::new_readonly(accounts.event_authority, false));

    // 15. Meteora program
    account_metas.push(AccountMeta::new_readonly(accounts.program, false));

    // Remaining accounts:
    //
    // - bin arrays
    // - Token-2022 transfer-hook accounts, when applicable
    account_metas.extend(remaining_accounts);

    let data = serialize_swap2_args(&Swap2Args {
        amount_in,
        min_amount_out,
        remaining_accounts_info,
    });

    Ok(Instruction {
        program_id: METEORA_DLMM_PROGRAM_ID,
        accounts: account_metas,
        data,
    })
}

pub fn invoke_swap2<'info>(
    accounts: Swap2Accounts,
    amount_in: u64,
    min_amount_out: u64,
    remaining_accounts: Vec<AccountMeta>,
    remaining_accounts_info: RemainingAccountsInfo,
    account_infos: &[AccountInfo<'info>],
    signer_seeds: &[&[u8]],
) -> Result<()> {
    let instruction = build_swap2_instruction(
        accounts,
        amount_in,
        min_amount_out,
        remaining_accounts,
        remaining_accounts_info,
    )?;

    invoke_signed(&instruction, account_infos, &[signer_seeds]).map_err(Into::into)
}
