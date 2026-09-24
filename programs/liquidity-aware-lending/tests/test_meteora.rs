use anchor_lang::{prelude::Pubkey, pubkey};
use solana_transaction::AccountMeta;
use spl_token_2022_interface::ID as TOKEN_2022_PROGRAM_ID;

use liquidity_aware_lending::integrations::meteora::{
    build_swap2_instruction, event_authority, serialize_swap2_args, AccountsType,
    RemainingAccountsInfo, RemainingAccountsSlice, Swap2Accounts, Swap2Args,
    METEORA_DLMM_PROGRAM_ID,
};

const TOKEN_PROGRAM_ID: Pubkey = pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

const MEMO_PROGRAM_ID: Pubkey = pubkey!("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr");

#[test]
fn test_swap2_discriminator() {
    let args = Swap2Args {
        amount_in: 1_000,
        min_amount_out: 900,
        remaining_accounts_info: RemainingAccountsInfo::default(),
    };

    let data = serialize_swap2_args(&args);

    assert_eq!(&data[..8], &[65, 75, 63, 76, 235, 91, 91, 136]);
}

#[test]
fn test_swap2_serializes_amounts() {
    let args = Swap2Args {
        amount_in: 1_000_000,
        min_amount_out: 950_000,
        remaining_accounts_info: RemainingAccountsInfo::default(),
    };

    let data = serialize_swap2_args(&args);

    assert_eq!(
        u64::from_le_bytes(data[8..16].try_into().unwrap()),
        1_000_000
    );

    assert_eq!(
        u64::from_le_bytes(data[16..24].try_into().unwrap()),
        950_000
    );
}

#[test]
fn test_swap2_empty_remaining_accounts_info() {
    let args = Swap2Args {
        amount_in: 1_000,
        min_amount_out: 900,
        remaining_accounts_info: RemainingAccountsInfo::default(),
    };

    let data = serialize_swap2_args(&args);

    // 8-byte discriminator
    // 8-byte amount_in
    // 8-byte min_amount_out
    // 4-byte Vec length = 0
    assert_eq!(data.len(), 28);

    assert_eq!(&data[24..28], &[0, 0, 0, 0]);
}

#[test]
fn test_swap2_remaining_accounts_info_serialization() {
    let args = Swap2Args {
        amount_in: 1_000,
        min_amount_out: 900,
        remaining_accounts_info: RemainingAccountsInfo {
            slices: vec![
                RemainingAccountsSlice {
                    accounts_type: AccountsType::TransferHookX,
                    length: 3,
                },
                RemainingAccountsSlice {
                    accounts_type: AccountsType::TransferHookY,
                    length: 2,
                },
            ],
        },
    };

    let data = serialize_swap2_args(&args);

    // Ensure serialization includes the vector length
    // and does not panic.
    assert!(data.len() > 28);
}

#[test]
fn test_event_authority_matches_meteora_derivation() {
    let expected =
        Pubkey::find_program_address(&[b"__event_authority"], &METEORA_DLMM_PROGRAM_ID).0;

    assert_eq!(event_authority(), expected);
}

#[test]
fn test_swap2_account_order_without_optional_accounts() {
    let accounts = Swap2Accounts {
        lb_pair: Pubkey::new_unique(),
        bin_array_bitmap_extension: None,
        reserve_x: Pubkey::new_unique(),
        reserve_y: Pubkey::new_unique(),
        user_token_in: Pubkey::new_unique(),
        user_token_out: Pubkey::new_unique(),
        token_x_mint: Pubkey::new_unique(),
        token_y_mint: Pubkey::new_unique(),
        oracle: Pubkey::new_unique(),
        host_fee_in: None,
        user: Pubkey::new_unique(),
        token_x_program: TOKEN_2022_PROGRAM_ID,
        token_y_program: TOKEN_PROGRAM_ID,
        memo_program: MEMO_PROGRAM_ID,
        event_authority: event_authority(),
        program: METEORA_DLMM_PROGRAM_ID,
    };

    let instruction = build_swap2_instruction(
        accounts.clone(),
        1_000,
        900,
        vec![],
        RemainingAccountsInfo::default(),
    )
    .expect("Swap2 instruction should build");

    assert_eq!(instruction.program_id, METEORA_DLMM_PROGRAM_ID);

    // Meteora keeps both optional account positions and uses the
    // DLMM program ID as a sentinel when the accounts are absent.
    //
    // 0  lb_pair
    // 1  bin_array_bitmap_extension
    // 2  reserve_x
    // 3  reserve_y
    // 4  user_token_in
    // 5  user_token_out
    // 6  token_x_mint
    // 7  token_y_mint
    // 8  oracle
    // 9  host_fee_in
    // 10 user
    // 11 token_x_program
    // 12 token_y_program
    // 13 memo_program
    // 14 event_authority
    // 15 program
    //
    // = 16 accounts.
    assert_eq!(instruction.accounts.len(), 16);

    assert_eq!(instruction.accounts[0].pubkey, accounts.lb_pair);

    assert_eq!(instruction.accounts[1].pubkey, METEORA_DLMM_PROGRAM_ID);

    assert_eq!(instruction.accounts[2].pubkey, accounts.reserve_x);

    assert_eq!(instruction.accounts[3].pubkey, accounts.reserve_y);

    assert_eq!(instruction.accounts[4].pubkey, accounts.user_token_in);

    assert_eq!(instruction.accounts[5].pubkey, accounts.user_token_out);

    assert_eq!(instruction.accounts[6].pubkey, accounts.token_x_mint);

    assert_eq!(instruction.accounts[7].pubkey, accounts.token_y_mint);

    assert_eq!(instruction.accounts[8].pubkey, accounts.oracle);

    assert_eq!(instruction.accounts[9].pubkey, METEORA_DLMM_PROGRAM_ID);

    assert_eq!(instruction.accounts[10].pubkey, accounts.user);

    assert!(instruction.accounts[10].is_signer);

    assert_eq!(instruction.accounts[11].pubkey, accounts.token_x_program);

    assert_eq!(instruction.accounts[12].pubkey, accounts.token_y_program);

    assert_eq!(instruction.accounts[13].pubkey, accounts.memo_program);

    assert_eq!(instruction.accounts[14].pubkey, accounts.event_authority);

    assert_eq!(instruction.accounts[15].pubkey, accounts.program);
}

#[test]
fn test_swap2_optional_accounts_are_inserted_in_correct_positions() {
    let bitmap_extension = Pubkey::new_unique();
    let host_fee = Pubkey::new_unique();

    let accounts = Swap2Accounts {
        lb_pair: Pubkey::new_unique(),
        bin_array_bitmap_extension: Some(bitmap_extension),
        reserve_x: Pubkey::new_unique(),
        reserve_y: Pubkey::new_unique(),
        user_token_in: Pubkey::new_unique(),
        user_token_out: Pubkey::new_unique(),
        token_x_mint: Pubkey::new_unique(),
        token_y_mint: Pubkey::new_unique(),
        oracle: Pubkey::new_unique(),
        host_fee_in: Some(host_fee),
        user: Pubkey::new_unique(),
        token_x_program: TOKEN_2022_PROGRAM_ID,
        token_y_program: TOKEN_PROGRAM_ID,
        memo_program: MEMO_PROGRAM_ID,
        event_authority: event_authority(),
        program: METEORA_DLMM_PROGRAM_ID,
    };

    let instruction = build_swap2_instruction(
        accounts.clone(),
        1_000,
        900,
        vec![],
        RemainingAccountsInfo::default(),
    )
    .expect("Swap2 instruction should build");

    assert_eq!(instruction.accounts.len(), 16);

    assert_eq!(instruction.accounts[0].pubkey, accounts.lb_pair);

    assert_eq!(instruction.accounts[1].pubkey, bitmap_extension);

    assert_eq!(instruction.accounts[2].pubkey, accounts.reserve_x);

    assert_eq!(instruction.accounts[3].pubkey, accounts.reserve_y);

    assert_eq!(instruction.accounts[8].pubkey, accounts.oracle);

    assert_eq!(instruction.accounts[9].pubkey, host_fee);

    assert_eq!(instruction.accounts[10].pubkey, accounts.user);

    assert!(instruction.accounts[10].is_signer);
}

#[test]
fn test_swap2_remaining_accounts_are_appended() {
    let accounts = Swap2Accounts {
        lb_pair: Pubkey::new_unique(),
        bin_array_bitmap_extension: None,
        reserve_x: Pubkey::new_unique(),
        reserve_y: Pubkey::new_unique(),
        user_token_in: Pubkey::new_unique(),
        user_token_out: Pubkey::new_unique(),
        token_x_mint: Pubkey::new_unique(),
        token_y_mint: Pubkey::new_unique(),
        oracle: Pubkey::new_unique(),
        host_fee_in: None,
        user: Pubkey::new_unique(),
        token_x_program: TOKEN_2022_PROGRAM_ID,
        token_y_program: TOKEN_PROGRAM_ID,
        memo_program: MEMO_PROGRAM_ID,
        event_authority: event_authority(),
        program: METEORA_DLMM_PROGRAM_ID,
    };

    let bin_array_one = Pubkey::new_unique();
    let bin_array_two = Pubkey::new_unique();

    let remaining_accounts = vec![
        AccountMeta::new(bin_array_one, false),
        AccountMeta::new(bin_array_two, false),
    ];

    let instruction = build_swap2_instruction(
        accounts,
        1_000,
        900,
        remaining_accounts,
        RemainingAccountsInfo::default(),
    )
    .expect("Swap2 instruction should build");

    assert_eq!(instruction.accounts.len(), 18);

    assert_eq!(instruction.accounts[16].pubkey, bin_array_one);

    assert_eq!(instruction.accounts[17].pubkey, bin_array_two);
}
