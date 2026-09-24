mod common;

use anchor_lang::{prelude::Pubkey, AccountDeserialize, InstructionData, ToAccountMetas};
use common::{
    associated_token_address, create_token_2022_ata, create_token_2022_mint, initialize_market,
    mint_token_2022, send_transaction, setup_lending_program, token_2022_account_amount,
};
use liquidity_aware_lending::constants::{POSITION_SEED, VAULT_SEED};
use liquidity_aware_lending::instruction::{
    Borrow as BorrowInstruction, DepositCollateral as DepositCollateralInstruction,
    Repay as RepayInstruction,
};
use solana_keypair::Keypair;
use solana_signer::Signer;
use spl_associated_token_account_interface::program::ID as ASSOCIATED_TOKEN_PROGRAM_ID;
use spl_token_2022_interface::ID as TOKEN_2022_PROGRAM_ID;

struct RepayTestContext {
    svm: litesvm::LiteSVM,
    payer: Keypair,
    program_id: Pubkey,
    market: Pubkey,
    position: Pubkey,
    collateral_mint: Keypair,
    debt_mint: Keypair,
    user_debt_account: Pubkey,
    vault_authority: Pubkey,
    debt_vault: Pubkey,
}

fn setup_repay_test() -> RepayTestContext {
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

    let user_collateral_account =
        associated_token_address(&payer.pubkey(), &collateral_mint.pubkey());

    let user_debt_account = associated_token_address(&payer.pubkey(), &debt_mint.pubkey());

    create_token_2022_ata(&mut svm, &payer, &payer.pubkey(), &collateral_mint.pubkey());

    create_token_2022_ata(&mut svm, &payer, &payer.pubkey(), &debt_mint.pubkey());

    let (vault_authority, _) =
        Pubkey::find_program_address(&[VAULT_SEED, market.as_ref()], &program_id);

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

    // 1,000 USDC liquidity in the debt vault.
    mint_token_2022(
        &mut svm,
        &payer,
        &debt_mint.pubkey(),
        &debt_vault,
        &payer,
        1_000_000_000,
    );

    let (position, _) = Pubkey::find_program_address(
        &[POSITION_SEED, market.as_ref(), payer.pubkey().as_ref()],
        &program_id,
    );

    // Deposit 1 collateral token.
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

    // Borrow 500 USDC so the position has debt to repay.
    let borrow_ix = BorrowInstruction { amount: 500_000 };

    let borrow_accounts = liquidity_aware_lending::accounts::Borrow {
        user: payer.pubkey(),
        market,
        position,
        collateral_mint: collateral_mint.pubkey(),
        debt_mint: debt_mint.pubkey(),
        user_debt_account,
        vault_authority,
        debt_vault,
        token_program: TOKEN_2022_PROGRAM_ID,
    };

    let borrow_instruction = anchor_lang::solana_program::instruction::Instruction::new_with_bytes(
        program_id,
        &borrow_ix.data(),
        borrow_accounts.to_account_metas(None),
    );

    let borrow_result = send_transaction(&mut svm, borrow_instruction, &payer, &[]);

    assert!(
        borrow_result.is_ok(),
        "borrow transaction failed: {:?}",
        borrow_result.err()
    );

    RepayTestContext {
        svm,
        payer,
        program_id,
        market,
        position,
        collateral_mint,
        debt_mint,
        user_debt_account,
        vault_authority,
        debt_vault,
    }
}

fn repay(ctx: &mut RepayTestContext, amount: u64) -> litesvm::types::TransactionResult {
    let repay_ix = RepayInstruction { amount };

    let repay_accounts = liquidity_aware_lending::accounts::Repay {
        user: ctx.payer.pubkey(),
        market: ctx.market,
        position: ctx.position,
        debt_mint: ctx.debt_mint.pubkey(),
        user_debt_account: ctx.user_debt_account,
        vault_authority: ctx.vault_authority,
        debt_vault: ctx.debt_vault,
        token_program: TOKEN_2022_PROGRAM_ID,
    };

    let repay_instruction = anchor_lang::solana_program::instruction::Instruction::new_with_bytes(
        ctx.program_id,
        &repay_ix.data(),
        repay_accounts.to_account_metas(None),
    );

    send_transaction(&mut ctx.svm, repay_instruction, &ctx.payer, &[])
}

#[test]
fn test_repay() {
    let mut ctx = setup_repay_test();

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

    let result = repay(&mut ctx, 200_000);

    assert!(
        result.is_ok(),
        "repay transaction failed: {:?}",
        result.err()
    );

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

    assert_eq!(user_debt_before - user_debt_after, 200_000);
    assert_eq!(debt_vault_after - debt_vault_before, 200_000);
}

#[test]
fn test_repay_reduces_debt() {
    let mut ctx = setup_repay_test();

    let result = repay(&mut ctx, 200_000);

    assert!(
        result.is_ok(),
        "repay transaction failed: {:?}",
        result.err()
    );

    let position_account = ctx
        .svm
        .get_account(&ctx.position)
        .expect("position account missing");

    let position =
        liquidity_aware_lending::state::Position::try_deserialize(&mut &position_account.data[..])
            .expect("failed to deserialize position");

    assert_eq!(position.debt_amount, 300_000);
}

#[test]
fn test_repay_multiple_times() {
    let mut ctx = setup_repay_test();

    let first = repay(&mut ctx, 100_000);
    assert!(first.is_ok(), "first repayment failed: {:?}", first.err());

    let second = repay(&mut ctx, 150_000);
    assert!(
        second.is_ok(),
        "second repayment failed: {:?}",
        second.err()
    );

    let position_account = ctx
        .svm
        .get_account(&ctx.position)
        .expect("position account missing");

    let position =
        liquidity_aware_lending::state::Position::try_deserialize(&mut &position_account.data[..])
            .expect("failed to deserialize position");

    assert_eq!(position.debt_amount, 250_000);
}

#[test]
fn test_repay_full_debt() {
    let mut ctx = setup_repay_test();

    let result = repay(&mut ctx, 500_000);

    assert!(result.is_ok(), "full repayment failed: {:?}", result.err());

    let position_account = ctx
        .svm
        .get_account(&ctx.position)
        .expect("position account missing");

    let position =
        liquidity_aware_lending::state::Position::try_deserialize(&mut &position_account.data[..])
            .expect("failed to deserialize position");

    assert_eq!(position.debt_amount, 0);
}

#[test]
fn test_repay_rejects_zero_amount() {
    let mut ctx = setup_repay_test();

    let result = repay(&mut ctx, 0);

    assert!(result.is_err(), "zero-amount repayment should have failed");
}

#[test]
fn test_repay_rejects_amount_greater_than_debt() {
    let mut ctx = setup_repay_test();

    let result = repay(&mut ctx, 500_001);

    assert!(
        result.is_err(),
        "repayment greater than outstanding debt should have failed"
    );
}
