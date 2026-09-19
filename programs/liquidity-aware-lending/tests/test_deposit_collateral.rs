mod common;

use anchor_lang::{AccountDeserialize, InstructionData, ToAccountMetas};

//use litesvm::LiteSVM;

use solana_keypair::Keypair;
use solana_signer::Signer;

use spl_associated_token_account_interface::program::ID as ASSOCIATED_TOKEN_PROGRAM_ID;
use spl_token_2022_interface::ID as TOKEN_2022_PROGRAM_ID;

use liquidity_aware_lending::{
    accounts::DepositCollateral, constants::*,
    instruction::DepositCollateral as DepositCollateralInstruction, state::Position,
};

use common::{
    associated_token_address, create_token_2022_ata, create_token_2022_mint,
    create_token_2022_transfer_fee_mint, initialize_market, mint_token_2022, send_transaction,
    setup_lending_program, token_2022_account_amount,
};

#[test]
fn test_deposit_collateral() {
    let program_id = liquidity_aware_lending::id();

    let (mut svm, payer) = setup_lending_program();

    // ---------------------------------------------------------------
    // Create Token-2022 mints
    // ---------------------------------------------------------------

    let collateral_mint = Keypair::new();
    let debt_mint = Keypair::new();

    let collateral_mint_authority = Keypair::new();
    let debt_mint_authority = Keypair::new();

    create_token_2022_mint(
        &mut svm,
        &payer,
        &collateral_mint,
        9,
        &collateral_mint_authority.pubkey(),
    );

    create_token_2022_mint(
        &mut svm,
        &payer,
        &debt_mint,
        6,
        &debt_mint_authority.pubkey(),
    );

    let collateral_mint_pubkey = collateral_mint.pubkey();
    let debt_mint_pubkey = debt_mint.pubkey();

    // ---------------------------------------------------------------
    // Initialize market
    // ---------------------------------------------------------------

    let dlmm_pool = anchor_lang::prelude::Pubkey::new_unique();

    let market = initialize_market(
        &mut svm,
        program_id,
        &payer,
        collateral_mint_pubkey,
        debt_mint_pubkey,
        dlmm_pool,
    );

    // ---------------------------------------------------------------
    // User collateral ATA
    // ---------------------------------------------------------------

    let user_collateral_account =
        associated_token_address(&payer.pubkey(), &collateral_mint_pubkey);

    create_token_2022_ata(&mut svm, &payer, &payer.pubkey(), &collateral_mint_pubkey);

    // ---------------------------------------------------------------
    // Vault authority + vault ATA
    // ---------------------------------------------------------------

    let (vault_authority, _) = anchor_lang::prelude::Pubkey::find_program_address(
        &[VAULT_SEED, market.as_ref()],
        &program_id,
    );

    let vault_collateral_account =
        associated_token_address(&vault_authority, &collateral_mint_pubkey);

    // ---------------------------------------------------------------
    // Mint collateral to user
    // ---------------------------------------------------------------

    let initial_collateral_amount: u64 = 1_000_000_000;

    mint_token_2022(
        &mut svm,
        &payer,
        &collateral_mint_pubkey,
        &user_collateral_account,
        &collateral_mint_authority,
        initial_collateral_amount,
    );

    let user_account_before = svm
        .get_account(&user_collateral_account)
        .expect("user collateral ATA should exist");

    assert_eq!(
        token_2022_account_amount(&user_account_before.data),
        initial_collateral_amount,
    );

    // ---------------------------------------------------------------
    // Position PDA
    // ---------------------------------------------------------------

    let (position, _) = anchor_lang::prelude::Pubkey::find_program_address(
        &[POSITION_SEED, market.as_ref(), payer.pubkey().as_ref()],
        &program_id,
    );

    // ---------------------------------------------------------------
    // Deposit
    // ---------------------------------------------------------------

    let deposit_amount: u64 = 500_000_000;

    let deposit_instruction = anchor_lang::solana_program::instruction::Instruction::new_with_bytes(
        program_id,
        &DepositCollateralInstruction {
            amount: deposit_amount,
        }
        .data(),
        DepositCollateral {
            user: payer.pubkey(),
            market,
            position,
            collateral_mint: collateral_mint_pubkey,
            user_collateral_account,
            vault_authority,
            vault_collateral_account,
            token_program: TOKEN_2022_PROGRAM_ID,
            associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
            system_program: anchor_lang::solana_program::system_program::ID,
        }
        .to_account_metas(None),
    );

    let result = send_transaction(&mut svm, deposit_instruction, &payer, &[]);

    assert!(
        result.is_ok(),
        "deposit transaction failed: {:?}",
        result.err()
    );

    // ---------------------------------------------------------------
    // Position assertions
    // ---------------------------------------------------------------

    let position_account = svm
        .get_account(&position)
        .expect("position account should exist");

    let mut position_data: &[u8] = &position_account.data;

    let position_state =
        Position::try_deserialize(&mut position_data).expect("failed to deserialize Position");

    assert_eq!(position_state.owner, payer.pubkey());
    assert_eq!(position_state.market, market);

    assert_eq!(
        position_state.collateral_amount, deposit_amount,
        "position collateral should equal the amount actually received",
    );

    assert_eq!(
        position_state.debt_amount, 0,
        "deposit should not create debt",
    );

    // ---------------------------------------------------------------
    // Vault assertions
    // ---------------------------------------------------------------

    let vault_account = svm
        .get_account(&vault_collateral_account)
        .expect("vault collateral ATA should exist");

    assert_eq!(
        token_2022_account_amount(&vault_account.data),
        deposit_amount,
        "vault should contain the deposited collateral",
    );

    // ---------------------------------------------------------------
    // User balance assertion
    // ---------------------------------------------------------------

    let user_account_after = svm
        .get_account(&user_collateral_account)
        .expect("user collateral ATA should still exist");

    assert_eq!(
        token_2022_account_amount(&user_account_after.data),
        initial_collateral_amount - deposit_amount,
        "user collateral balance should decrease by the deposited amount",
    );
}

#[test]
fn test_deposit_collateral_rejects_zero_amount() {
    let program_id = liquidity_aware_lending::id();

    let (mut svm, payer) = setup_lending_program();

    // ---------------------------------------------------------------
    // Create Token-2022 mints
    // ---------------------------------------------------------------

    let collateral_mint = Keypair::new();
    let debt_mint = Keypair::new();

    let collateral_mint_authority = Keypair::new();
    let debt_mint_authority = Keypair::new();

    create_token_2022_mint(
        &mut svm,
        &payer,
        &collateral_mint,
        9,
        &collateral_mint_authority.pubkey(),
    );

    create_token_2022_mint(
        &mut svm,
        &payer,
        &debt_mint,
        6,
        &debt_mint_authority.pubkey(),
    );

    let collateral_mint_pubkey = collateral_mint.pubkey();
    let debt_mint_pubkey = debt_mint.pubkey();

    // ---------------------------------------------------------------
    // Initialize market
    // ---------------------------------------------------------------

    let dlmm_pool = anchor_lang::prelude::Pubkey::new_unique();

    let market = initialize_market(
        &mut svm,
        program_id,
        &payer,
        collateral_mint_pubkey,
        debt_mint_pubkey,
        dlmm_pool,
    );

    // ---------------------------------------------------------------
    // User collateral ATA
    // ---------------------------------------------------------------

    let user_collateral_account =
        associated_token_address(&payer.pubkey(), &collateral_mint_pubkey);

    create_token_2022_ata(&mut svm, &payer, &payer.pubkey(), &collateral_mint_pubkey);

    // ---------------------------------------------------------------
    // Vault authority + vault ATA
    // ---------------------------------------------------------------

    let (vault_authority, _) = anchor_lang::prelude::Pubkey::find_program_address(
        &[VAULT_SEED, market.as_ref()],
        &program_id,
    );

    let vault_collateral_account =
        associated_token_address(&vault_authority, &collateral_mint_pubkey);

    // ---------------------------------------------------------------
    // Position PDA
    // ---------------------------------------------------------------

    let (position, _) = anchor_lang::prelude::Pubkey::find_program_address(
        &[POSITION_SEED, market.as_ref(), payer.pubkey().as_ref()],
        &program_id,
    );

    // ---------------------------------------------------------------
    // Attempt zero deposit
    // ---------------------------------------------------------------

    let deposit_instruction = anchor_lang::solana_program::instruction::Instruction::new_with_bytes(
        program_id,
        &DepositCollateralInstruction { amount: 0 }.data(),
        DepositCollateral {
            user: payer.pubkey(),
            market,
            position,
            collateral_mint: collateral_mint_pubkey,
            user_collateral_account,
            vault_authority,
            vault_collateral_account,
            token_program: TOKEN_2022_PROGRAM_ID,
            associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
            system_program: anchor_lang::solana_program::system_program::ID,
        }
        .to_account_metas(None),
    );

    let result = send_transaction(&mut svm, deposit_instruction, &payer, &[]);

    // ---------------------------------------------------------------
    // Assert rejection
    // ---------------------------------------------------------------

    assert!(
        result.is_err(),
        "zero collateral deposit should be rejected",
    );
}

#[test]
fn test_deposit_collateral_rejects_insufficient_balance() {
    let program_id = liquidity_aware_lending::id();

    let (mut svm, payer) = setup_lending_program();

    // ---------------------------------------------------------------
    // Create Token-2022 mints
    // ---------------------------------------------------------------

    let collateral_mint = Keypair::new();
    let debt_mint = Keypair::new();

    let collateral_mint_authority = Keypair::new();
    let debt_mint_authority = Keypair::new();

    create_token_2022_mint(
        &mut svm,
        &payer,
        &collateral_mint,
        9,
        &collateral_mint_authority.pubkey(),
    );

    create_token_2022_mint(
        &mut svm,
        &payer,
        &debt_mint,
        6,
        &debt_mint_authority.pubkey(),
    );

    let collateral_mint_pubkey = collateral_mint.pubkey();
    let debt_mint_pubkey = debt_mint.pubkey();

    // ---------------------------------------------------------------
    // Initialize market
    // ---------------------------------------------------------------

    let dlmm_pool = anchor_lang::prelude::Pubkey::new_unique();

    let market = initialize_market(
        &mut svm,
        program_id,
        &payer,
        collateral_mint_pubkey,
        debt_mint_pubkey,
        dlmm_pool,
    );

    // ---------------------------------------------------------------
    // User collateral ATA
    // ---------------------------------------------------------------

    let user_collateral_account =
        associated_token_address(&payer.pubkey(), &collateral_mint_pubkey);

    create_token_2022_ata(&mut svm, &payer, &payer.pubkey(), &collateral_mint_pubkey);

    // ---------------------------------------------------------------
    // Vault authority + vault ATA
    // ---------------------------------------------------------------

    let (vault_authority, _) = anchor_lang::prelude::Pubkey::find_program_address(
        &[VAULT_SEED, market.as_ref()],
        &program_id,
    );

    let vault_collateral_account =
        associated_token_address(&vault_authority, &collateral_mint_pubkey);

    // ---------------------------------------------------------------
    // Position PDA
    // ---------------------------------------------------------------

    let (position, _) = anchor_lang::prelude::Pubkey::find_program_address(
        &[POSITION_SEED, market.as_ref(), payer.pubkey().as_ref()],
        &program_id,
    );

    // ---------------------------------------------------------------
    // Give the user less collateral than they will attempt to deposit
    // ---------------------------------------------------------------

    let user_balance: u64 = 100_000_000;
    let deposit_amount: u64 = 200_000_000;

    mint_token_2022(
        &mut svm,
        &payer,
        &collateral_mint_pubkey,
        &user_collateral_account,
        &collateral_mint_authority,
        user_balance,
    );

    let user_account_before = svm
        .get_account(&user_collateral_account)
        .expect("user collateral ATA should exist");

    assert_eq!(
        token_2022_account_amount(&user_account_before.data),
        user_balance,
    );

    // ---------------------------------------------------------------
    // Attempt to deposit more than the user's balance
    // ---------------------------------------------------------------

    let deposit_instruction = anchor_lang::solana_program::instruction::Instruction::new_with_bytes(
        program_id,
        &DepositCollateralInstruction {
            amount: deposit_amount,
        }
        .data(),
        DepositCollateral {
            user: payer.pubkey(),
            market,
            position,
            collateral_mint: collateral_mint_pubkey,
            user_collateral_account,
            vault_authority,
            vault_collateral_account,
            token_program: TOKEN_2022_PROGRAM_ID,
            associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
            system_program: anchor_lang::solana_program::system_program::ID,
        }
        .to_account_metas(None),
    );

    let result = send_transaction(&mut svm, deposit_instruction, &payer, &[]);

    // ---------------------------------------------------------------
    // Assert rejection
    // ---------------------------------------------------------------

    assert!(
        result.is_err(),
        "deposit exceeding user balance should be rejected",
    );

    // ---------------------------------------------------------------
    // Assert user's balance was not changed
    // ---------------------------------------------------------------

    let user_account_after = svm
        .get_account(&user_collateral_account)
        .expect("user collateral ATA should still exist");

    assert_eq!(
        token_2022_account_amount(&user_account_after.data),
        user_balance,
        "failed deposit must not change user balance",
    );

    // ---------------------------------------------------------------
    // Assert vault was not funded
    // ---------------------------------------------------------------

    assert!(
        svm.get_account(&vault_collateral_account).is_none(),
        "failed deposit must not create or fund the vault ATA",
    );

    // ---------------------------------------------------------------
    // Assert position was not created
    // ---------------------------------------------------------------

    assert!(
        svm.get_account(&position).is_none(),
        "failed deposit must not create the position account",
    );
}

#[test]
fn test_deposit_collateral_rejects_wrong_mint() {
    let program_id = liquidity_aware_lending::id();

    let (mut svm, payer) = setup_lending_program();

    // ---------------------------------------------------------------
    // Create the market's collateral mint and another unrelated mint
    // ---------------------------------------------------------------

    let collateral_mint = Keypair::new();
    let wrong_mint = Keypair::new();
    let debt_mint = Keypair::new();

    let collateral_mint_authority = Keypair::new();
    let wrong_mint_authority = Keypair::new();
    let debt_mint_authority = Keypair::new();

    create_token_2022_mint(
        &mut svm,
        &payer,
        &collateral_mint,
        9,
        &collateral_mint_authority.pubkey(),
    );

    create_token_2022_mint(
        &mut svm,
        &payer,
        &wrong_mint,
        9,
        &wrong_mint_authority.pubkey(),
    );

    create_token_2022_mint(
        &mut svm,
        &payer,
        &debt_mint,
        6,
        &debt_mint_authority.pubkey(),
    );

    let collateral_mint_pubkey = collateral_mint.pubkey();
    let wrong_mint_pubkey = wrong_mint.pubkey();
    let debt_mint_pubkey = debt_mint.pubkey();

    // ---------------------------------------------------------------
    // Initialize market using the legitimate collateral mint
    // ---------------------------------------------------------------

    let dlmm_pool = anchor_lang::prelude::Pubkey::new_unique();

    let market = initialize_market(
        &mut svm,
        program_id,
        &payer,
        collateral_mint_pubkey,
        debt_mint_pubkey,
        dlmm_pool,
    );

    // ---------------------------------------------------------------
    // Create ATA for the WRONG mint
    // ---------------------------------------------------------------

    let wrong_user_account = associated_token_address(&payer.pubkey(), &wrong_mint_pubkey);

    create_token_2022_ata(&mut svm, &payer, &payer.pubkey(), &wrong_mint_pubkey);

    mint_token_2022(
        &mut svm,
        &payer,
        &wrong_mint_pubkey,
        &wrong_user_account,
        &wrong_mint_authority,
        1_000_000_000,
    );

    // ---------------------------------------------------------------
    // Correct vault authority
    // ---------------------------------------------------------------

    let (vault_authority, _) = anchor_lang::prelude::Pubkey::find_program_address(
        &[VAULT_SEED, market.as_ref()],
        &program_id,
    );

    let vault_collateral_account =
        associated_token_address(&vault_authority, &collateral_mint_pubkey);

    // ---------------------------------------------------------------
    // Correct position PDA
    // ---------------------------------------------------------------

    let (position, _) = anchor_lang::prelude::Pubkey::find_program_address(
        &[POSITION_SEED, market.as_ref(), payer.pubkey().as_ref()],
        &program_id,
    );

    // ---------------------------------------------------------------
    // Attempt deposit using wrong mint/accounts
    // ---------------------------------------------------------------

    let deposit_instruction = anchor_lang::solana_program::instruction::Instruction::new_with_bytes(
        program_id,
        &DepositCollateralInstruction {
            amount: 500_000_000,
        }
        .data(),
        DepositCollateral {
            user: payer.pubkey(),
            market,
            position,
            collateral_mint: wrong_mint_pubkey,
            user_collateral_account: wrong_user_account,
            vault_authority,
            vault_collateral_account,
            token_program: TOKEN_2022_PROGRAM_ID,
            associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
            system_program: anchor_lang::solana_program::system_program::ID,
        }
        .to_account_metas(None),
    );

    let result = send_transaction(&mut svm, deposit_instruction, &payer, &[]);

    assert!(
        result.is_err(),
        "deposit using an unrelated collateral mint should be rejected",
    );

    assert!(
        svm.get_account(&position).is_none(),
        "failed deposit must not create the position",
    );
}

#[test]
fn test_deposit_collateral_rejects_wrong_position_pda() {
    let program_id = liquidity_aware_lending::id();

    let (mut svm, payer) = setup_lending_program();

    let collateral_mint = Keypair::new();
    let debt_mint = Keypair::new();

    let collateral_mint_authority = Keypair::new();
    let debt_mint_authority = Keypair::new();

    create_token_2022_mint(
        &mut svm,
        &payer,
        &collateral_mint,
        9,
        &collateral_mint_authority.pubkey(),
    );

    create_token_2022_mint(
        &mut svm,
        &payer,
        &debt_mint,
        6,
        &debt_mint_authority.pubkey(),
    );

    let collateral_mint_pubkey = collateral_mint.pubkey();
    let debt_mint_pubkey = debt_mint.pubkey();

    let market = initialize_market(
        &mut svm,
        program_id,
        &payer,
        collateral_mint_pubkey,
        debt_mint_pubkey,
        anchor_lang::prelude::Pubkey::new_unique(),
    );

    let user_collateral_account =
        associated_token_address(&payer.pubkey(), &collateral_mint_pubkey);

    create_token_2022_ata(&mut svm, &payer, &payer.pubkey(), &collateral_mint_pubkey);

    mint_token_2022(
        &mut svm,
        &payer,
        &collateral_mint_pubkey,
        &user_collateral_account,
        &collateral_mint_authority,
        1_000_000_000,
    );

    let (vault_authority, _) = anchor_lang::prelude::Pubkey::find_program_address(
        &[VAULT_SEED, market.as_ref()],
        &program_id,
    );

    let vault_collateral_account =
        associated_token_address(&vault_authority, &collateral_mint_pubkey);

    // Deliberately derive the WRONG position PDA.
    let wrong_user = Keypair::new();

    let (wrong_position, _) = anchor_lang::prelude::Pubkey::find_program_address(
        &[POSITION_SEED, market.as_ref(), wrong_user.pubkey().as_ref()],
        &program_id,
    );

    let instruction = anchor_lang::solana_program::instruction::Instruction::new_with_bytes(
        program_id,
        &DepositCollateralInstruction {
            amount: 500_000_000,
        }
        .data(),
        DepositCollateral {
            user: payer.pubkey(),
            market,
            position: wrong_position,
            collateral_mint: collateral_mint_pubkey,
            user_collateral_account,
            vault_authority,
            vault_collateral_account,
            token_program: TOKEN_2022_PROGRAM_ID,
            associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
            system_program: anchor_lang::solana_program::system_program::ID,
        }
        .to_account_metas(None),
    );

    let result = send_transaction(&mut svm, instruction, &payer, &[]);

    assert!(
        result.is_err(),
        "deposit using the wrong position PDA should be rejected",
    );

    assert!(
        svm.get_account(&wrong_position).is_none(),
        "wrong position PDA must not be initialized",
    );
}

#[test]
fn test_deposit_collateral_accumulates_repeated_deposits() {
    let program_id = liquidity_aware_lending::id();

    let (mut svm, payer) = setup_lending_program();

    let collateral_mint = Keypair::new();
    let debt_mint = Keypair::new();

    let collateral_mint_authority = Keypair::new();
    let debt_mint_authority = Keypair::new();

    create_token_2022_mint(
        &mut svm,
        &payer,
        &collateral_mint,
        9,
        &collateral_mint_authority.pubkey(),
    );

    create_token_2022_mint(
        &mut svm,
        &payer,
        &debt_mint,
        6,
        &debt_mint_authority.pubkey(),
    );

    let collateral_mint_pubkey = collateral_mint.pubkey();
    let debt_mint_pubkey = debt_mint.pubkey();

    let market = initialize_market(
        &mut svm,
        program_id,
        &payer,
        collateral_mint_pubkey,
        debt_mint_pubkey,
        anchor_lang::prelude::Pubkey::new_unique(),
    );

    let user_collateral_account =
        associated_token_address(&payer.pubkey(), &collateral_mint_pubkey);

    create_token_2022_ata(&mut svm, &payer, &payer.pubkey(), &collateral_mint_pubkey);

    let initial_balance = 1_000_000_000;

    mint_token_2022(
        &mut svm,
        &payer,
        &collateral_mint_pubkey,
        &user_collateral_account,
        &collateral_mint_authority,
        initial_balance,
    );

    let (vault_authority, _) = anchor_lang::prelude::Pubkey::find_program_address(
        &[VAULT_SEED, market.as_ref()],
        &program_id,
    );

    let vault_collateral_account =
        associated_token_address(&vault_authority, &collateral_mint_pubkey);

    let (position, _) = anchor_lang::prelude::Pubkey::find_program_address(
        &[POSITION_SEED, market.as_ref(), payer.pubkey().as_ref()],
        &program_id,
    );

    let first_deposit = 300_000_000;
    let second_deposit = 200_000_000;

    // ---------------------------------------------------------------
    // First deposit
    // ---------------------------------------------------------------

    let first_instruction = anchor_lang::solana_program::instruction::Instruction::new_with_bytes(
        program_id,
        &DepositCollateralInstruction {
            amount: first_deposit,
        }
        .data(),
        DepositCollateral {
            user: payer.pubkey(),
            market,
            position,
            collateral_mint: collateral_mint_pubkey,
            user_collateral_account,
            vault_authority,
            vault_collateral_account,
            token_program: TOKEN_2022_PROGRAM_ID,
            associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
            system_program: anchor_lang::solana_program::system_program::ID,
        }
        .to_account_metas(None),
    );

    let first_result = send_transaction(&mut svm, first_instruction, &payer, &[]);

    assert!(
        first_result.is_ok(),
        "first deposit should succeed: {:?}",
        first_result.err()
    );

    // ---------------------------------------------------------------
    // Second deposit using the SAME position
    // ---------------------------------------------------------------

    let second_instruction = anchor_lang::solana_program::instruction::Instruction::new_with_bytes(
        program_id,
        &DepositCollateralInstruction {
            amount: second_deposit,
        }
        .data(),
        DepositCollateral {
            user: payer.pubkey(),
            market,
            position,
            collateral_mint: collateral_mint_pubkey,
            user_collateral_account,
            vault_authority,
            vault_collateral_account,
            token_program: TOKEN_2022_PROGRAM_ID,
            associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
            system_program: anchor_lang::solana_program::system_program::ID,
        }
        .to_account_metas(None),
    );

    let second_result = send_transaction(&mut svm, second_instruction, &payer, &[]);

    assert!(
        second_result.is_ok(),
        "second deposit should succeed: {:?}",
        second_result.err()
    );

    // ---------------------------------------------------------------
    // Assert accumulated position
    // ---------------------------------------------------------------

    let position_account = svm
        .get_account(&position)
        .expect("position account should exist");

    let mut position_data: &[u8] = &position_account.data;

    let position_state =
        Position::try_deserialize(&mut position_data).expect("failed to deserialize Position");

    assert_eq!(
        position_state.collateral_amount,
        first_deposit + second_deposit,
        "position collateral should accumulate",
    );

    // ---------------------------------------------------------------
    // Assert accumulated vault balance
    // ---------------------------------------------------------------

    let vault_account = svm
        .get_account(&vault_collateral_account)
        .expect("vault account should exist");

    assert_eq!(
        token_2022_account_amount(&vault_account.data),
        first_deposit + second_deposit,
        "vault balance should accumulate",
    );

    // ---------------------------------------------------------------
    // Assert remaining user balance
    // ---------------------------------------------------------------

    let user_account = svm
        .get_account(&user_collateral_account)
        .expect("user collateral ATA should exist");

    assert_eq!(
        token_2022_account_amount(&user_account.data),
        initial_balance - first_deposit - second_deposit,
        "user balance should decrease by both deposits",
    );
}

#[test]
fn test_deposit_collateral_accounts_for_transfer_fee() {
    let program_id = liquidity_aware_lending::id();

    let (mut svm, payer) = setup_lending_program();

    // ---------------------------------------------------------------
    // Create a Token-2022 collateral mint with a 50 bps transfer fee
    // ---------------------------------------------------------------

    let collateral_mint = Keypair::new();
    let debt_mint = Keypair::new();

    let collateral_mint_authority = Keypair::new();
    let debt_mint_authority = Keypair::new();

    let transfer_fee_bps: u16 = 50;
    let maximum_fee: u64 = 1_000_000_000;

    create_token_2022_transfer_fee_mint(
        &mut svm,
        &payer,
        &collateral_mint,
        9,
        &collateral_mint_authority.pubkey(),
        transfer_fee_bps,
        maximum_fee,
    );

    create_token_2022_mint(
        &mut svm,
        &payer,
        &debt_mint,
        6,
        &debt_mint_authority.pubkey(),
    );

    let collateral_mint_pubkey = collateral_mint.pubkey();
    let debt_mint_pubkey = debt_mint.pubkey();

    // ---------------------------------------------------------------
    // Initialize market
    // ---------------------------------------------------------------

    let market = initialize_market(
        &mut svm,
        program_id,
        &payer,
        collateral_mint_pubkey,
        debt_mint_pubkey,
        anchor_lang::prelude::Pubkey::new_unique(),
    );

    // ---------------------------------------------------------------
    // User collateral ATA
    // ---------------------------------------------------------------

    let user_collateral_account =
        associated_token_address(&payer.pubkey(), &collateral_mint_pubkey);

    create_token_2022_ata(&mut svm, &payer, &payer.pubkey(), &collateral_mint_pubkey);

    // ---------------------------------------------------------------
    // Vault authority + vault ATA
    // ---------------------------------------------------------------

    let (vault_authority, _) = anchor_lang::prelude::Pubkey::find_program_address(
        &[VAULT_SEED, market.as_ref()],
        &program_id,
    );

    let vault_collateral_account =
        associated_token_address(&vault_authority, &collateral_mint_pubkey);

    // ---------------------------------------------------------------
    // Position PDA
    // ---------------------------------------------------------------

    let (position, _) = anchor_lang::prelude::Pubkey::find_program_address(
        &[POSITION_SEED, market.as_ref(), payer.pubkey().as_ref()],
        &program_id,
    );

    // ---------------------------------------------------------------
    // Mint collateral to the user
    // ---------------------------------------------------------------

    let initial_balance: u64 = 1_000_000_000;

    mint_token_2022(
        &mut svm,
        &payer,
        &collateral_mint_pubkey,
        &user_collateral_account,
        &collateral_mint_authority,
        initial_balance,
    );

    // ---------------------------------------------------------------
    // Deposit
    // ---------------------------------------------------------------

    let deposit_amount: u64 = 500_000_000;

    let instruction = anchor_lang::solana_program::instruction::Instruction::new_with_bytes(
        program_id,
        &DepositCollateralInstruction {
            amount: deposit_amount,
        }
        .data(),
        DepositCollateral {
            user: payer.pubkey(),
            market,
            position,
            collateral_mint: collateral_mint_pubkey,
            user_collateral_account,
            vault_authority,
            vault_collateral_account,
            token_program: TOKEN_2022_PROGRAM_ID,
            associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
            system_program: anchor_lang::solana_program::system_program::ID,
        }
        .to_account_metas(None),
    );

    let result = send_transaction(&mut svm, instruction, &payer, &[]);

    assert!(
        result.is_ok(),
        "deposit with transfer-fee mint should succeed: {:?}",
        result.err()
    );

    // ---------------------------------------------------------------
    // Read actual amount received by vault
    // ---------------------------------------------------------------

    let vault_account = svm
        .get_account(&vault_collateral_account)
        .expect("vault collateral ATA should exist");

    let vault_balance = token_2022_account_amount(&vault_account.data);

    // 50 bps = 0.5%
    let expected_fee = deposit_amount * transfer_fee_bps as u64 / 10_000;

    let expected_received = deposit_amount - expected_fee;

    assert_eq!(
        vault_balance, expected_received,
        "vault should receive the deposit amount minus the Token-2022 transfer fee",
    );

    // ---------------------------------------------------------------
    // Position must record ACTUAL received collateral
    // ---------------------------------------------------------------

    let position_account = svm
        .get_account(&position)
        .expect("position account should exist");

    let mut position_data: &[u8] = &position_account.data;

    let position_state =
        Position::try_deserialize(&mut position_data).expect("failed to deserialize Position");

    assert_eq!(
        position_state.collateral_amount, expected_received,
        "position collateral must equal actual vault receipt, not requested amount",
    );

    // ---------------------------------------------------------------
    // User balance should decrease by the full transfer amount
    // ---------------------------------------------------------------

    let user_account = svm
        .get_account(&user_collateral_account)
        .expect("user collateral ATA should exist");

    assert_eq!(
        token_2022_account_amount(&user_account.data),
        initial_balance - deposit_amount,
        "user should be debited the full transfer amount",
    );
}
