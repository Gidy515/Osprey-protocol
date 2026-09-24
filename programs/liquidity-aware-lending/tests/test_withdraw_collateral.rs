mod common;

use anchor_lang::{prelude::Pubkey, AccountDeserialize, InstructionData, ToAccountMetas};

use common::{
    associated_token_address, create_token_2022_ata, create_token_2022_mint,
    create_token_2022_transfer_fee_mint, initialize_liquidity_risk, initialize_market,
    mint_token_2022, send_transaction, setup_lending_program, token_2022_account_amount,
};

use liquidity_aware_lending::{
    constants::{POSITION_SEED, VAULT_SEED},
    instruction::{
        Borrow as BorrowInstruction, DepositCollateral as DepositCollateralInstruction,
        WithdrawCollateral as WithdrawCollateralInstruction,
    },
    state::Position,
};

use solana_keypair::Keypair;
use solana_signer::Signer;

use spl_associated_token_account_interface::program::ID as ASSOCIATED_TOKEN_PROGRAM_ID;

use spl_token_2022_interface::{
    extension::ExtensionType, state::Mint, ID as TOKEN_2022_PROGRAM_ID,
};

struct WithdrawTestContext {
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

fn setup_withdraw_test() -> WithdrawTestContext {
    let (mut svm, payer) = setup_lending_program();

    let program_id = liquidity_aware_lending::id();

    // ---------------------------------------------------------
    // Create collateral + debt mints.
    // ---------------------------------------------------------

    let collateral_mint = Keypair::new();

    create_token_2022_mint(&mut svm, &payer, &collateral_mint, 9, &payer.pubkey());

    let debt_mint = Keypair::new();

    create_token_2022_mint(&mut svm, &payer, &debt_mint, 6, &payer.pubkey());

    // ---------------------------------------------------------
    // Initialize lending market.
    // ---------------------------------------------------------

    let dlmm_pool = Pubkey::new_unique();

    let market = initialize_market(
        &mut svm,
        program_id,
        &payer,
        collateral_mint.pubkey(),
        debt_mint.pubkey(),
        dlmm_pool,
    );

    // ---------------------------------------------------------
    // Publish a fresh liquidity-risk snapshot.
    //
    // $5,000 reference liquidation
    // -> $5,000 executable output
    //
    // execution LTV = 65%
    // issuer ceiling = 60%
    // effective LTV = 60%
    // ---------------------------------------------------------

    let risk_snapshot = initialize_liquidity_risk(
        &mut svm,
        program_id,
        &payer,
        market,
        5_000_000_000,
        5_000_000_000,
    );

    // ---------------------------------------------------------
    // User token accounts.
    // ---------------------------------------------------------

    let user_collateral_account =
        associated_token_address(&payer.pubkey(), &collateral_mint.pubkey());

    create_token_2022_ata(&mut svm, &payer, &payer.pubkey(), &collateral_mint.pubkey());

    let user_debt_account = associated_token_address(&payer.pubkey(), &debt_mint.pubkey());

    create_token_2022_ata(&mut svm, &payer, &payer.pubkey(), &debt_mint.pubkey());

    // ---------------------------------------------------------
    // Protocol vault authority + vaults.
    // ---------------------------------------------------------

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

    // ---------------------------------------------------------
    // Fund user with 1 collateral token.
    // ---------------------------------------------------------

    mint_token_2022(
        &mut svm,
        &payer,
        &collateral_mint.pubkey(),
        &user_collateral_account,
        &payer,
        1_000_000_000,
    );

    // ---------------------------------------------------------
    // Fund protocol debt vault.
    // ---------------------------------------------------------

    mint_token_2022(
        &mut svm,
        &payer,
        &debt_mint.pubkey(),
        &debt_vault,
        &payer,
        1_000_000_000,
    );

    // ---------------------------------------------------------
    // Position PDA.
    // ---------------------------------------------------------

    let (position, _) = Pubkey::find_program_address(
        &[POSITION_SEED, market.as_ref(), payer.pubkey().as_ref()],
        &program_id,
    );

    // ---------------------------------------------------------
    // Deposit 1 collateral token.
    // ---------------------------------------------------------

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

fn withdraw(ctx: &mut WithdrawTestContext, amount: u64) -> Result<(), String> {
    let instruction = anchor_lang::solana_program::instruction::Instruction::new_with_bytes(
        ctx.program_id,
        &WithdrawCollateralInstruction { amount }.data(),
        liquidity_aware_lending::accounts::WithdrawCollateral {
            user: ctx.payer.pubkey(),
            market: ctx.market,
            position: ctx.position,
            risk_snapshot: ctx.risk_snapshot,
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
            risk_snapshot: ctx.risk_snapshot,
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

    // 1 collateral = $1.
    //
    // Borrow $0.50.
    //
    // Effective LTV = 60%, so this is valid.
    borrow(&mut ctx, 500_000).unwrap();

    // Remaining collateral = $0.90.
    //
    // $0.90 * 60% = $0.54 borrowing capacity.
    //
    // $0.50 debt therefore remains healthy.
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

    // Remaining collateral after this withdrawal:
    //
    // $0.70
    //
    // Effective capacity:
    //
    // $0.70 * 60% = $0.42
    //
    // Existing debt = $0.50
    //
    // Therefore this must fail.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        withdraw(&mut ctx, 300_000_000).unwrap();
    }));

    assert!(
        result.is_err(),
        "withdraw should reject withdrawal that violates effective LTV"
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

    // ---------------------------------------------------------
    // Token-2022 collateral with 50 bps transfer fee.
    // ---------------------------------------------------------

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

    // ---------------------------------------------------------
    // Market + risk snapshot.
    // ---------------------------------------------------------

    let dlmm_pool = Pubkey::new_unique();

    let market = initialize_market(
        &mut svm,
        program_id,
        &payer,
        collateral_mint.pubkey(),
        debt_mint.pubkey(),
        dlmm_pool,
    );

    let risk_snapshot = initialize_liquidity_risk(
        &mut svm,
        program_id,
        &payer,
        market,
        5_000_000_000,
        5_000_000_000,
    );

    // ---------------------------------------------------------
    // PDAs.
    // ---------------------------------------------------------

    let position = Pubkey::find_program_address(
        &[POSITION_SEED, market.as_ref(), payer.pubkey().as_ref()],
        &program_id,
    )
    .0;

    let vault_authority =
        Pubkey::find_program_address(&[VAULT_SEED, market.as_ref()], &program_id).0;

    // ---------------------------------------------------------
    // Collateral token accounts.
    // ---------------------------------------------------------

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

    // ---------------------------------------------------------
    // Mint 1 collateral token to user.
    // ---------------------------------------------------------

    mint_token_2022(
        &mut svm,
        &payer,
        &collateral_mint.pubkey(),
        &user_collateral_account,
        &payer,
        1_000_000_000,
    );

    // ---------------------------------------------------------
    // Deposit 1 token.
    //
    // With the 50 bps transfer fee the vault actually receives:
    //
    // 995,000,000
    // ---------------------------------------------------------

    let deposit_data = DepositCollateralInstruction {
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

    assert_eq!(vault_before, 995_000_000);

    assert_eq!(user_before, 0);

    // ---------------------------------------------------------
    // Withdraw 0.4 collateral tokens.
    // ---------------------------------------------------------

    let withdraw_amount = 400_000_000u64;

    let withdraw_data = WithdrawCollateralInstruction {
        amount: withdraw_amount,
    }
    .data();

    let withdraw_accounts = liquidity_aware_lending::accounts::WithdrawCollateral {
        user: payer.pubkey(),
        market,
        position,
        risk_snapshot,
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

    // Vault is debited by the full source amount.
    assert_eq!(vault_after, 595_000_000);

    // User receives 398,000,000 because the
    // 400,000,000 withdrawal incurs a 50 bps fee.
    assert_eq!(user_after, 398_000_000);

    let position_account = svm.get_account(&position).unwrap();

    let position_state = Position::try_deserialize(&mut &position_account.data[..]).unwrap();

    // Position tracks the source amount removed
    // from the vault, not the user's net receipt.
    assert_eq!(position_state.collateral_amount, 595_000_000);
}
