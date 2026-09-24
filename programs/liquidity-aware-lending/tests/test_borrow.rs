mod common;

use anchor_lang::{prelude::Pubkey, InstructionData, ToAccountMetas};
use common::{
    associated_token_address, create_token_2022_ata, create_token_2022_mint,
    initialize_liquidity_risk, initialize_market, mint_token_2022, send_transaction,
    setup_lending_program, token_2022_account_amount,
};
use liquidity_aware_lending::constants::VAULT_SEED;
use liquidity_aware_lending::instruction::{
    Borrow as BorrowInstruction, DepositCollateral as DepositCollateralInstruction,
};
use solana_keypair::Keypair;
use solana_signer::Signer;
use spl_associated_token_account_interface::program::ID as ASSOCIATED_TOKEN_PROGRAM_ID;
use spl_token_2022_interface::ID as TOKEN_2022_PROGRAM_ID;

struct BorrowTestContext {
    svm: litesvm::LiteSVM,
    payer: Keypair,
    program_id: Pubkey,
    market: Pubkey,
    risk_snapshot: Pubkey,
    position: Pubkey,
    collateral_mint: Keypair,
    debt_mint: Keypair,
    user_collateral_account: Pubkey,
    user_debt_account: Pubkey,
    vault_authority: Pubkey,
    vault_collateral_account: Pubkey,
    debt_vault: Pubkey,
}

fn setup_borrow_test(debt_liquidity: u64) -> BorrowTestContext {
    let (mut svm, payer) = setup_lending_program();
    let program_id = liquidity_aware_lending::id();

    let collateral_mint = Keypair::new();
    let debt_mint = Keypair::new();

    create_token_2022_mint(&mut svm, &payer, &collateral_mint, 9, &payer.pubkey());

    create_token_2022_mint(&mut svm, &payer, &debt_mint, 6, &payer.pubkey());

    let market = initialize_market(
        &mut svm,
        program_id,
        &payer,
        collateral_mint.pubkey(),
        debt_mint.pubkey(),
        Keypair::new().pubkey(),
    );

    let risk_snapshot = initialize_liquidity_risk(
        &mut svm,
        program_id,
        &payer,
        market,
        5_000_000_000,
        5_000_000_000,
    );

    let user_collateral_account =
        associated_token_address(&payer.pubkey(), &collateral_mint.pubkey());

    let user_debt_account = associated_token_address(&payer.pubkey(), &debt_mint.pubkey());

    create_token_2022_ata(&mut svm, &payer, &payer.pubkey(), &collateral_mint.pubkey());

    create_token_2022_ata(&mut svm, &payer, &payer.pubkey(), &debt_mint.pubkey());

    let (vault_authority, _) = anchor_lang::solana_program::pubkey::Pubkey::find_program_address(
        &[VAULT_SEED, market.as_ref()],
        &program_id,
    );

    let vault_collateral_account =
        associated_token_address(&vault_authority, &collateral_mint.pubkey());

    let debt_vault = associated_token_address(&vault_authority, &debt_mint.pubkey());

    create_token_2022_ata(
        &mut svm,
        &payer,
        &vault_authority,
        &collateral_mint.pubkey(),
    );

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
        debt_liquidity,
    );

    let (position, _) = anchor_lang::solana_program::pubkey::Pubkey::find_program_address(
        &[b"position", market.as_ref(), payer.pubkey().as_ref()],
        &program_id,
    );

    let deposit_ix = DepositCollateralInstruction {
        amount: 1_000_000_000,
    };

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
    };

    let deposit_instruction = anchor_lang::solana_program::instruction::Instruction::new_with_bytes(
        program_id,
        &deposit_ix.data(),
        deposit_accounts.to_account_metas(None),
    );

    let deposit_result = send_transaction(&mut svm, deposit_instruction, &payer, &[]);

    assert!(
        deposit_result.is_ok(),
        "deposit transaction failed: {:?}",
        deposit_result.err()
    );

    BorrowTestContext {
        svm,
        payer,
        program_id,
        market,
        risk_snapshot,
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

fn borrow(ctx: &mut BorrowTestContext, amount: u64) -> litesvm::types::TransactionResult {
    let borrow_ix = BorrowInstruction { amount };

    let borrow_accounts = liquidity_aware_lending::accounts::Borrow {
        user: ctx.payer.pubkey(),
        market: ctx.market,
        position: ctx.position,
        risk_snapshot: ctx.risk_snapshot,

        collateral_mint: ctx.collateral_mint.pubkey(),
        debt_mint: ctx.debt_mint.pubkey(),

        user_debt_account: ctx.user_debt_account,
        vault_authority: ctx.vault_authority,
        debt_vault: ctx.debt_vault,
        token_program: TOKEN_2022_PROGRAM_ID,
    };

    let borrow_instruction = anchor_lang::solana_program::instruction::Instruction::new_with_bytes(
        ctx.program_id,
        &borrow_ix.data(),
        borrow_accounts.to_account_metas(None),
    );

    send_transaction(&mut ctx.svm, borrow_instruction, &ctx.payer, &[])
}

#[test]
fn test_borrow() {
    let mut ctx = setup_borrow_test(1_000_000);

    let result = borrow(&mut ctx, 500_000);

    assert!(
        result.is_ok(),
        "borrow transaction failed: {:?}",
        result.err()
    );

    let user_debt_data = ctx
        .svm
        .get_account(&ctx.user_debt_account)
        .expect("user debt account missing");

    let debt_vault_data = ctx
        .svm
        .get_account(&ctx.debt_vault)
        .expect("debt vault missing");

    assert_eq!(token_2022_account_amount(&user_debt_data.data), 500_000);

    assert_eq!(token_2022_account_amount(&debt_vault_data.data), 500_000);
}

#[test]
fn test_borrow_rejects_zero_amount() {
    let mut ctx = setup_borrow_test(1_000_000);

    let result = borrow(&mut ctx, 0);

    assert!(result.is_err(), "zero-amount borrow should have failed");
}

#[test]
fn test_borrow_rejects_amount_exceeding_ltv() {
    let mut ctx = setup_borrow_test(1_000_000);

    let result = borrow(&mut ctx, 700_000);

    assert!(
        result.is_err(),
        "borrow exceeding max LTV should have failed"
    );
}

#[test]
fn test_borrow_accumulates_debt() {
    let mut ctx = setup_borrow_test(1_000_000);

    let first_borrow = borrow(&mut ctx, 300_000);

    assert!(
        first_borrow.is_ok(),
        "first borrow failed: {:?}",
        first_borrow.err()
    );

    let second_borrow = borrow(&mut ctx, 200_000);

    assert!(
        second_borrow.is_ok(),
        "second borrow failed: {:?}",
        second_borrow.err()
    );

    let user_debt_data = ctx
        .svm
        .get_account(&ctx.user_debt_account)
        .expect("user debt account missing");

    let debt_vault_data = ctx
        .svm
        .get_account(&ctx.debt_vault)
        .expect("debt vault missing");

    assert_eq!(token_2022_account_amount(&user_debt_data.data), 500_000);

    assert_eq!(token_2022_account_amount(&debt_vault_data.data), 500_000);
}

#[test]
fn test_borrow_rejects_wrong_debt_mint() {
    let mut ctx = setup_borrow_test(1_000_000);

    let borrow_ix = BorrowInstruction { amount: 500_000 };

    let borrow_accounts = liquidity_aware_lending::accounts::Borrow {
        user: ctx.payer.pubkey(),
        market: ctx.market,
        position: ctx.position,
        risk_snapshot: ctx.risk_snapshot,
        collateral_mint: ctx.collateral_mint.pubkey(),

        // Deliberately wrong: market expects ctx.debt_mint.
        debt_mint: ctx.collateral_mint.pubkey(),

        user_debt_account: ctx.user_debt_account,
        vault_authority: ctx.vault_authority,
        debt_vault: ctx.debt_vault,
        token_program: TOKEN_2022_PROGRAM_ID,
    };

    let instruction = anchor_lang::solana_program::instruction::Instruction::new_with_bytes(
        ctx.program_id,
        &borrow_ix.data(),
        borrow_accounts.to_account_metas(None),
    );

    let result = send_transaction(&mut ctx.svm, instruction, &ctx.payer, &[]);

    assert!(
        result.is_err(),
        "borrow with wrong debt mint should have failed"
    );
}

#[test]
fn test_borrow_rejects_insufficient_debt_liquidity() {
    let mut ctx = setup_borrow_test(400_000);

    let result = borrow(&mut ctx, 500_000);

    assert!(
        result.is_err(),
        "borrow should have failed because the debt vault lacks liquidity"
    );
}

#[test]
fn test_borrow_failure_does_not_change_balances() {
    let mut ctx = setup_borrow_test(1_000_000);

    let user_debt_before = token_2022_account_amount(
        &ctx.svm
            .get_account(&ctx.user_debt_account)
            .expect("user debt account missing")
            .data,
    );

    let debt_vault_before = token_2022_account_amount(
        &ctx.svm
            .get_account(&ctx.debt_vault)
            .expect("debt vault missing")
            .data,
    );

    let result = borrow(&mut ctx, 700_000);

    assert!(result.is_err(), "borrow exceeding LTV should have failed");

    let user_debt_after = token_2022_account_amount(
        &ctx.svm
            .get_account(&ctx.user_debt_account)
            .expect("user debt account missing")
            .data,
    );

    let debt_vault_after = token_2022_account_amount(
        &ctx.svm
            .get_account(&ctx.debt_vault)
            .expect("debt vault missing")
            .data,
    );

    assert_eq!(user_debt_after, user_debt_before);
    assert_eq!(debt_vault_after, debt_vault_before);
}
