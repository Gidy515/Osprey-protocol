mod common;

use anchor_lang::{prelude::Pubkey, AccountDeserialize, InstructionData, ToAccountMetas};

use common::{
    associated_token_address, create_token_2022_ata, create_token_2022_mint,
    create_token_2022_transfer_fee_mint, initialize_market, mint_token_2022, send_transaction,
    setup_lending_program, token_2022_account_amount,
};

use spl_token_2022_interface::{
    extension::ExtensionType, state::Mint, ID as TOKEN_2022_PROGRAM_ID,
};

use liquidity_aware_lending::{
    constants::VAULT_SEED,
    instruction::{
        Borrow as BorrowInstruction, DepositCollateral as DepositCollateralInstruction,
        WithdrawCollateral as WithdrawCollateralInstruction,
    },
    state::Position,
    POSITION_SEED,
};

use solana_keypair::Keypair;
use solana_signer::Signer;

use spl_associated_token_account_interface::program::ID as ASSOCIATED_TOKEN_PROGRAM_ID;

struct WithdrawTestContext {
    svm: litesvm::LiteSVM,
    payer: Keypair,
    program_id: Pubkey,
    market: Pubkey,
    position: Pubkey,

    collateral_mint: Keypair,
    debt_mint: Keypair,

    user_collateral_account: Pubkey,
    user_debt_account: Pubkey,

    vault_authority: Pubkey,
    vault_collateral_account: Pubkey,
    debt_vault: Pubkey,
}

fn setup_withdraw_test() -> WithdrawTestContext {
    let (mut svm, payer) = setup_lending_program();
    let program_id = liquidity_aware_lending::id();

    let collateral_mint = Keypair::new();
    create_token_2022_mint(&mut svm, &payer, &collateral_mint, 9, &payer.pubkey());

    let debt_mint = Keypair::new();
    create_token_2022_mint(&mut svm, &payer, &debt_mint, 6, &payer.pubkey());

    let dlmm_pool = Pubkey::new_unique();

    let market = initialize_market(
        &mut svm,
        program_id,
        &payer,
        collateral_mint.pubkey(),
        debt_mint.pubkey(),
        dlmm_pool,
    );

    let user_collateral_account =
        associated_token_address(&payer.pubkey(), &collateral_mint.pubkey());

    create_token_2022_ata(&mut svm, &payer, &payer.pubkey(), &collateral_mint.pubkey());

    let user_debt_account = associated_token_address(&payer.pubkey(), &debt_mint.pubkey());

    create_token_2022_ata(&mut svm, &payer, &payer.pubkey(), &debt_mint.pubkey());

    let (vault_authority, _) =
        Pubkey::find_program_address(&[VAULT_SEED, market.as_ref()], &program_id);

    let vault_collateral_account =
        associated_token_address(&vault_authority, &collateral_mint.pubkey());

    create_token_2022_ata(
        &mut svm,
        &payer,
        &vault_authority,
        &collateral_mint.pubkey(),
    );

    let debt_vault = associated_token_address(&vault_authority, &debt_mint.pubkey());

    create_token_2022_ata(&mut svm, &payer, &vault_authority, &debt_mint.pubkey());

    mint_token_2022(
        &mut svm,
        &payer,
        &collateral_mint.pubkey(),
        &user_collateral_account,
        &payer,
        1_000_000_000,
    );

    mint_token_2022(
        &mut svm,
        &payer,
        &debt_mint.pubkey(),
        &debt_vault,
        &payer,
        1_000_000_000,
    );

    let (position, _) = Pubkey::find_program_address(
        &[
            liquidity_aware_lending::constants::POSITION_SEED,
            market.as_ref(),
            payer.pubkey().as_ref(),
        ],
        &program_id,
    );

    let deposit_instruction = anchor_lang::solana_program::instruction::Instruction::new_with_bytes(
        program_id,
        &DepositCollateralInstruction {
            amount: 1_000_000_000,
        }
        .data(),
        liquidity_aware_lending::accounts::DepositCollateral {
            user: payer.pubkey(),
            market,
            position,
            collateral_mint: collateral_mint.pubkey(),
            user_collateral_account,
            vault_authority,
            vault_collateral_account,
            token_program: TOKEN_2022_PROGRAM_ID,
            associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
            system_program: anchor_lang::solana_program::system_program::ID,
        }
        .to_account_metas(None),
    );

    send_transaction(&mut svm, deposit_instruction, &payer, &[]).unwrap();

    WithdrawTestContext {
        svm,
        payer,
        program_id,
        market,
        position,
        collateral_mint,
        debt_mint,
        user_collateral_account,
        user_debt_account,
        vault_authority,
        vault_collateral_account,
        debt_vault,
    }
}

fn withdraw(ctx: &mut WithdrawTestContext, amount: u64) -> Result<(), String> {
    let instruction = anchor_lang::solana_program::instruction::Instruction::new_with_bytes(
        ctx.program_id,
        &WithdrawCollateralInstruction { amount }.data(),
        liquidity_aware_lending::accounts::WithdrawCollateral {
            user: ctx.payer.pubkey(),
            market: ctx.market,
            position: ctx.position,
            collateral_mint: ctx.collateral_mint.pubkey(),
            user_collateral_account: ctx.user_collateral_account,
            vault_authority: ctx.vault_authority,
            vault_collateral_account: ctx.vault_collateral_account,
            token_program: TOKEN_2022_PROGRAM_ID,
        }
        .to_account_metas(None),
    );

    send_transaction(&mut ctx.svm, instruction, &ctx.payer, &[])
        .map(|_| ())
        .map_err(|err| format!("{err:?}"))
}

fn borrow(ctx: &mut WithdrawTestContext, amount: u64) -> Result<(), String> {
    let instruction = anchor_lang::solana_program::instruction::Instruction::new_with_bytes(
        ctx.program_id,
        &BorrowInstruction { amount }.data(),
        liquidity_aware_lending::accounts::Borrow {
            user: ctx.payer.pubkey(),
            market: ctx.market,
            position: ctx.position,
            collateral_mint: ctx.collateral_mint.pubkey(),
            debt_mint: ctx.debt_mint.pubkey(),
            user_debt_account: ctx.user_debt_account,
            vault_authority: ctx.vault_authority,
            debt_vault: ctx.debt_vault,
            token_program: TOKEN_2022_PROGRAM_ID,
        }
        .to_account_metas(None),
    );

    send_transaction(&mut ctx.svm, instruction, &ctx.payer, &[])
        .map(|_| ())
        .map_err(|err| format!("{err:?}"))
}

#[test]
fn test_withdraw_collateral() {
    let mut ctx = setup_withdraw_test();

    let user_before = token_2022_account_amount(
        &ctx.svm
            .get_account(&ctx.user_collateral_account)
            .unwrap()
            .data,
    );

    let vault_before = token_2022_account_amount(
        &ctx.svm
            .get_account(&ctx.vault_collateral_account)
            .unwrap()
            .data,
    );

    withdraw(&mut ctx, 400_000_000).unwrap();

    let user_after = token_2022_account_amount(
        &ctx.svm
            .get_account(&ctx.user_collateral_account)
            .unwrap()
            .data,
    );

    let vault_after = token_2022_account_amount(
        &ctx.svm
            .get_account(&ctx.vault_collateral_account)
            .unwrap()
            .data,
    );

    assert_eq!(user_after - user_before, 400_000_000);
    assert_eq!(vault_before - vault_after, 400_000_000);

    let position_account = ctx.svm.get_account(&ctx.position).unwrap();

    let position = Position::try_deserialize(&mut &position_account.data[..])
        .expect("failed to deserialize Position");

    assert_eq!(position.collateral_amount, 600_000_000);
    assert_eq!(position.debt_amount, 0);
}

#[test]
fn test_withdraw_collateral_with_debt() {
    let mut ctx = setup_withdraw_test();

    borrow(&mut ctx, 500_000).unwrap();

    withdraw(&mut ctx, 100_000_000).unwrap();

    let position_account = ctx.svm.get_account(&ctx.position).unwrap();

    let position = Position::try_deserialize(&mut &position_account.data[..])
        .expect("failed to deserialize Position");

    assert_eq!(position.collateral_amount, 900_000_000);
    assert_eq!(position.debt_amount, 500_000);
}

#[test]
fn test_withdraw_collateral_rejects_zero_amount() {
    let mut ctx = setup_withdraw_test();

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        withdraw(&mut ctx, 0).unwrap();
    }));

    assert!(result.is_err(), "withdraw should reject zero amount");
}

#[test]
fn test_withdraw_collateral_rejects_excessive_amount() {
    let mut ctx = setup_withdraw_test();

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        withdraw(&mut ctx, 1_000_000_001).unwrap();
    }));

    assert!(
        result.is_err(),
        "withdraw should reject amount greater than collateral"
    );
}

#[test]
fn test_withdraw_collateral_rejects_ltv_violation() {
    let mut ctx = setup_withdraw_test();

    borrow(&mut ctx, 500_000).unwrap();

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        withdraw(&mut ctx, 300_000_000).unwrap();
    }));

    assert!(
        result.is_err(),
        "withdraw should reject withdrawal that violates LTV"
    );
}

#[test]
fn test_withdraw_collateral_does_not_change_position_on_failure() {
    let mut ctx = setup_withdraw_test();

    borrow(&mut ctx, 500_000).unwrap();

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        withdraw(&mut ctx, 300_000_000).unwrap();
    }));

    assert!(result.is_err());

    let position_account = ctx.svm.get_account(&ctx.position).unwrap();

    let position = Position::try_deserialize(&mut &position_account.data[..])
        .expect("failed to deserialize Position");

    assert_eq!(position.collateral_amount, 1_000_000_000);
    assert_eq!(position.debt_amount, 500_000);
}

#[test]
fn test_withdraw_collateral_accounts_for_transfer_fee() {
    let (mut svm, payer) = setup_lending_program();

    let program_id = liquidity_aware_lending::ID;

    let collateral_mint = Keypair::new();
    let debt_mint = Keypair::new();

    create_token_2022_transfer_fee_mint(
        &mut svm,
        &payer,
        &collateral_mint,
        9,
        &payer.pubkey(),
        50,
        1_000_000_000,
    );

    let mint_account = svm
        .get_account(&collateral_mint.pubkey())
        .expect("transfer-fee mint account should exist");

    println!("mint owner: {}", mint_account.owner);
    println!("mint data length: {}", mint_account.data.len());

    let expected_mint_space =
        ExtensionType::try_calculate_account_len::<Mint>(&[ExtensionType::TransferFeeConfig])
            .expect("failed to calculate expected mint size");

    println!("expected mint data length: {}", expected_mint_space);

    assert_eq!(
        mint_account.data.len(),
        expected_mint_space,
        "transfer-fee mint was allocated with an unexpected size"
    );

    let dlmm_pool = Pubkey::new_unique();

    let market = initialize_market(
        &mut svm,
        program_id,
        &payer,
        collateral_mint.pubkey(),
        debt_mint.pubkey(),
        dlmm_pool,
    );

    let position = Pubkey::find_program_address(
        &[POSITION_SEED, market.as_ref(), payer.pubkey().as_ref()],
        &program_id,
    )
    .0;

    let vault_authority =
        Pubkey::find_program_address(&[VAULT_SEED, market.as_ref()], &program_id).0;

    let user_collateral_account =
        associated_token_address(&payer.pubkey(), &collateral_mint.pubkey());

    let vault_collateral_account =
        associated_token_address(&vault_authority, &collateral_mint.pubkey());

    create_token_2022_ata(&mut svm, &payer, &payer.pubkey(), &collateral_mint.pubkey());

    create_token_2022_ata(
        &mut svm,
        &payer,
        &vault_authority,
        &collateral_mint.pubkey(),
    );

    // Mint 1 token to the user.
    mint_token_2022(
        &mut svm,
        &payer,
        &collateral_mint.pubkey(),
        &user_collateral_account,
        &payer,
        1_000_000_000,
    );

    // Deposit 1 token.
    let deposit_data = liquidity_aware_lending::instruction::DepositCollateral {
        amount: 1_000_000_000,
    }
    .data();

    let deposit_accounts = liquidity_aware_lending::accounts::DepositCollateral {
        user: payer.pubkey(),
        market,
        position,
        collateral_mint: collateral_mint.pubkey(),
        user_collateral_account,
        vault_authority,
        vault_collateral_account,
        token_program: TOKEN_2022_PROGRAM_ID,
        associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
        system_program: anchor_lang::solana_program::system_program::ID,
    }
    .to_account_metas(None);

    let deposit_instruction = anchor_lang::solana_program::instruction::Instruction::new_with_bytes(
        program_id,
        &deposit_data,
        deposit_accounts,
    );

    send_transaction(&mut svm, deposit_instruction, &payer, &[]).unwrap();

    let vault_before =
        token_2022_account_amount(&svm.get_account(&vault_collateral_account).unwrap().data);

    let user_before =
        token_2022_account_amount(&svm.get_account(&user_collateral_account).unwrap().data);

    // The 1-token deposit incurs a 0.5% transfer fee.
    // Therefore the vault receives 995,000,000 base units.
    assert_eq!(vault_before, 995_000_000);
    assert_eq!(user_before, 0);

    // Withdraw 0.4 tokens from the vault.
    let withdraw_amount = 400_000_000u64;

    let withdraw_data = liquidity_aware_lending::instruction::WithdrawCollateral {
        amount: withdraw_amount,
    }
    .data();

    let withdraw_accounts = liquidity_aware_lending::accounts::WithdrawCollateral {
        user: payer.pubkey(),
        market,
        position,
        collateral_mint: collateral_mint.pubkey(),
        user_collateral_account,
        vault_authority,
        vault_collateral_account,
        token_program: TOKEN_2022_PROGRAM_ID,
    }
    .to_account_metas(None);

    let withdraw_instruction =
        anchor_lang::solana_program::instruction::Instruction::new_with_bytes(
            program_id,
            &withdraw_data,
            withdraw_accounts,
        );

    send_transaction(&mut svm, withdraw_instruction, &payer, &[]).unwrap();

    let vault_after =
        token_2022_account_amount(&svm.get_account(&vault_collateral_account).unwrap().data);

    let user_after =
        token_2022_account_amount(&svm.get_account(&user_collateral_account).unwrap().data);

    // The vault is debited by the full source amount.
    assert_eq!(vault_after, 595_000_000);

    // The user receives 398,000,000 because the 400,000,000
    // source amount incurs a 50 bps (0.5%) transfer fee.
    assert_eq!(user_after, 398_000_000);

    let position_account = svm.get_account(&position).unwrap();

    let position_state = Position::try_deserialize(&mut &position_account.data[..]).unwrap();

    // Position tracks the source amount removed from the vault,
    // not the net amount received by the user.
    assert_eq!(position_state.collateral_amount, 595_000_000);
}
