use anchor_lang::AccountDeserialize;
use anchor_lang::{InstructionData, ToAccountMetas};
use liquidity_aware_lending::state::Position;
use litesvm::types::TransactionResult;
use solana_account::Account;
use solana_keypair::Keypair;
use solana_signer::Signer;

use liquidity_aware_lending::{
    accounts::{Borrow, Liquidate},
    constants::{POSITION_SEED, VAULT_SEED},
    instruction::{Borrow as BorrowInstruction, Liquidate as LiquidateInstruction},
    integrations::meteora::{event_authority, METEORA_DLMM_PROGRAM_ID},
};

mod common;

use common::{
    associated_token_address, create_token_2022_ata, create_token_2022_mint,
    initialize_liquidity_risk, initialize_market, mint_token_2022, send_transaction,
    setup_lending_program,
};

const MOCK_RESERVE_AUTHORITY_SEED: &[u8] = b"mock-reserve-authority";

const MEMO_PROGRAM_ID: anchor_lang::prelude::Pubkey =
    anchor_lang::prelude::pubkey!("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr");

#[test]
fn test_liquidate_reaches_meteora_cpi() {
    let (mut svm, payer) = setup_lending_program();
    let program_id = liquidity_aware_lending::id();

    // ---------------------------------------------------------
    // 1. Create the collateral and debt mints.
    //
    // Both are Token-2022 in this isolated CPI test. The actual
    // selected PreStocks collateral is also Token-2022.
    // ---------------------------------------------------------

    let collateral_mint = Keypair::new();
    let debt_mint = Keypair::new();

    create_token_2022_mint(&mut svm, &payer, &collateral_mint, 9, &payer.pubkey());

    create_token_2022_mint(&mut svm, &payer, &debt_mint, 6, &payer.pubkey());

    // ---------------------------------------------------------
    let lb_pair = anchor_lang::prelude::Pubkey::new_unique();
    let reserve_x = anchor_lang::prelude::Pubkey::new_unique();
    let oracle = anchor_lang::prelude::Pubkey::new_unique();

    // -------------------------------------------------------------------------
    // The mock reserve authority is a PDA owned by mock-meteora.
    //
    // The mock Meteora program will use this PDA as the authority of its
    // Token-2022 USDC reserve and sign the nested Token-2022 transfer CPI.
    // -------------------------------------------------------------------------

    let (mock_reserve_authority, _) = anchor_lang::prelude::Pubkey::find_program_address(
        &[MOCK_RESERVE_AUTHORITY_SEED],
        &METEORA_DLMM_PROGRAM_ID,
    );

    // -------------------------------------------------------------------------
    // Fake Meteora accounts.
    //
    // These don't need to be real DLMM accounts because the mock only needs
    // them to exist for the CPI account validation.
    // -------------------------------------------------------------------------

    for address in [lb_pair, reserve_x, oracle] {
        svm.set_account(
            address.into(),
            Account {
                lamports: 1_000_000,
                data: vec![],
                owner: METEORA_DLMM_PROGRAM_ID.into(),
                executable: false,
                rent_epoch: 0,
            },
        )
        .expect("failed to create fake Meteora account");
    }

    // -------------------------------------------------------------------------
    // reserve_y is different.
    //
    // It must be a REAL Token-2022 USDC token account because mock-meteora
    // performs a nested Token-2022 transfer:
    //
    //     reserve_y -> debt_vault
    //
    // Its authority is the mock reserve-authority PDA.
    // -------------------------------------------------------------------------

    let reserve_y = associated_token_address(&mock_reserve_authority, &debt_mint.pubkey());

    create_token_2022_ata(
        &mut svm,
        &payer,
        &mock_reserve_authority,
        &debt_mint.pubkey(),
    );

    // Fund the mock reserve with enough USDC for the simulated swap.
    //
    // The liquidation test sells 2.5 collateral tokens,
    // so reserve_y must contain at least 2.5 USDC.
    mint_token_2022(
        &mut svm,
        &payer,
        &debt_mint.pubkey(),
        &reserve_y,
        &payer,
        2_500_000,
    );

    // ---------------------------------------------------------
    // 3. Initialize the lending market.
    //
    // The shared test helper initializes:
    //
    //     1 collateral = 1 USDC
    //
    // Therefore 10 collateral = $10.
    // ---------------------------------------------------------

    let market = initialize_market(
        &mut svm,
        program_id,
        &payer,
        collateral_mint.pubkey(),
        debt_mint.pubkey(),
        lb_pair,
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
    // 4. Derive the protocol vault authority.
    // ---------------------------------------------------------

    let (vault_authority, _) = anchor_lang::prelude::Pubkey::find_program_address(
        &[VAULT_SEED, market.as_ref()],
        &program_id,
    );

    // ---------------------------------------------------------
    // 5. Create the user's collateral ATA and give the user
    //    enough collateral to create a position.
    // ---------------------------------------------------------

    let user_collateral_account =
        associated_token_address(&payer.pubkey(), &collateral_mint.pubkey());

    create_token_2022_ata(&mut svm, &payer, &payer.pubkey(), &collateral_mint.pubkey());

    mint_token_2022(
        &mut svm,
        &payer,
        &collateral_mint.pubkey(),
        &user_collateral_account,
        &payer,
        10_000_000_000, // 10 collateral tokens
    );

    // ---------------------------------------------------------
    // 6. Deposit collateral.
    //
    // This creates the Position PDA and vault collateral ATA,
    // giving us the exact on-chain accounts expected by
    // Liquidate.
    // ---------------------------------------------------------

    let (position, _) = anchor_lang::prelude::Pubkey::find_program_address(
        &[POSITION_SEED, market.as_ref(), payer.pubkey().as_ref()],
        &program_id,
    );

    let vault_collateral_account =
        associated_token_address(&vault_authority, &collateral_mint.pubkey());

    let deposit_instruction = anchor_lang::solana_program::instruction::Instruction::new_with_bytes(
        program_id,
        &liquidity_aware_lending::instruction::DepositCollateral {
            amount: 10_000_000_000,
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
            token_program: spl_token_2022_interface::ID,
            associated_token_program: anchor_spl::associated_token::ID,
            system_program: anchor_lang::solana_program::system_program::ID,
        }
        .to_account_metas(None),
    );

    let deposit_result = send_transaction(&mut svm, deposit_instruction, &payer, &[]);

    assert!(
        deposit_result.is_ok(),
        "deposit transaction failed: {:?}",
        deposit_result.err()
    );

    // ---------------------------------------------------------
    // 7. Create and fund the debt vault.
    // ---------------------------------------------------------

    let debt_vault = associated_token_address(&vault_authority, &debt_mint.pubkey());

    create_token_2022_ata(&mut svm, &payer, &vault_authority, &debt_mint.pubkey());

    // Give the protocol enough USDC liquidity to satisfy the borrow.
    mint_token_2022(
        &mut svm,
        &payer,
        &debt_mint.pubkey(),
        &debt_vault,
        &payer,
        6_000_000, // 6 USDC
    );

    // ---------------------------------------------------------
    // 8. Borrow against the collateral.
    //
    // At the initial $1 price:
    //
    //     collateral value = $10
    //     debt             = $6
    //     LTV              = 60%
    //
    // This is below the 65% maximum borrow LTV.
    // ---------------------------------------------------------

    let user_debt_account = associated_token_address(&payer.pubkey(), &debt_mint.pubkey());

    create_token_2022_ata(&mut svm, &payer, &payer.pubkey(), &debt_mint.pubkey());

    let borrow_instruction = anchor_lang::solana_program::instruction::Instruction::new_with_bytes(
        program_id,
        &BorrowInstruction { amount: 6_000_000 }.data(),
        Borrow {
            user: payer.pubkey(),
            market,
            position,
            risk_snapshot,
            collateral_mint: collateral_mint.pubkey(),
            debt_mint: debt_mint.pubkey(),
            user_debt_account,
            vault_authority,
            debt_vault,
            token_program: spl_token_2022_interface::ID,
        }
        .to_account_metas(None),
    );

    let borrow_result = send_transaction(&mut svm, borrow_instruction, &payer, &[]);

    assert!(
        borrow_result.is_ok(),
        "borrow transaction failed: {:?}",
        borrow_result.err()
    );

    // ---------------------------------------------------------
    // 9. Simulate a collateral price drop.
    //
    // The shared test market starts at:
    //
    //     1 collateral = 1 USDC
    //
    // The position is therefore:
    //
    //     $6 debt / $10 collateral = 60% LTV
    //
    // We now move the test price to $0.80:
    //
    //     $6 debt / $8 collateral = 75% LTV
    //
    // The liquidation threshold is 70%, so the position is now
    // genuinely liquidatable.
    //
    // This directly modifies test state only. Production code
    // never mutates the market price this way.
    // ---------------------------------------------------------

    let mut market_account = svm.get_account(&market).expect("market account missing");

    // Anchor account discriminator: 8 bytes
    // collateral_mint:             32 bytes
    // debt_mint:                   32 bytes
    //
    // Therefore collateral_price_usdc starts at byte 72.
    let collateral_price_offset = 8 + 32 + 32;

    market_account.data[collateral_price_offset..collateral_price_offset + 8]
        .copy_from_slice(&800_000u64.to_le_bytes());

    svm.set_account(market.into(), market_account)
        .expect("failed to update market price");

    // ---------------------------------------------------------
    // 10. Create the liquidator's collateral ATA.
    //
    // The current liquidate implementation does not transfer
    // liquidation proceeds yet, but the account is required by
    // the instruction interface.
    // ---------------------------------------------------------

    let liquidator = Keypair::new();

    svm.airdrop(&liquidator.pubkey(), 2_000_000_000)
        .expect("failed to fund liquidator");

    let liquidator_collateral_account =
        associated_token_address(&liquidator.pubkey(), &collateral_mint.pubkey());

    create_token_2022_ata(
        &mut svm,
        &payer,
        &liquidator.pubkey(),
        &collateral_mint.pubkey(),
    );

    // ---------------------------------------------------------
    // 11. Create the deterministic Meteora event authority.
    // ---------------------------------------------------------

    let meteora_event_authority = event_authority();

    svm.set_account(
        meteora_event_authority.into(),
        Account {
            lamports: 1_000_000,
            data: vec![],
            owner: anchor_lang::prelude::system_program::ID.into(),
            executable: false,
            rent_epoch: 0,
        },
    )
    .expect("failed to create Meteora event authority account");

    // Memo is only passed through to the mock in this test.
    svm.set_account(
        MEMO_PROGRAM_ID.into(),
        Account {
            lamports: 1_000_000,
            data: vec![],
            owner: anchor_lang::prelude::system_program::ID.into(),
            executable: false,
            rent_epoch: 0,
        },
    )
    .expect("failed to create memo program account");

    // ---------------------------------------------------------
    // 12. Build the liquidation instruction.
    //
    // One collateral token is supplied to Swap2. The mock does
    // not execute a swap, so min_amount_out is simply non-zero
    // to satisfy the current instruction validation.
    // ---------------------------------------------------------

    let debt_vault_before = common::token_2022_account_amount(
        &svm.get_account(&debt_vault)
            .expect("debt vault should exist")
            .data,
    );

    let liquidate_data = LiquidateInstruction {
        collateral_to_sell: 2_500_000_000,
        min_amount_out: 2_500_000,
    }
    .data();

    println!("liquidate discriminator: {:?}", &liquidate_data[..8]);

    let mut liquidate_accounts = Liquidate {
        liquidator: liquidator.pubkey(),
        market,
        position,
        risk_snapshot,
        collateral_mint: collateral_mint.pubkey(),
        liquidator_collateral_account,
        vault_authority,
        vault_collateral_account,
        debt_mint: debt_mint.pubkey(),
        debt_vault,
        lb_pair,
        reserve_x,
        reserve_y,
        oracle,
        token_x_mint: collateral_mint.pubkey(),
        token_y_mint: debt_mint.pubkey(),
        meteora_event_authority,
        meteora_program: METEORA_DLMM_PROGRAM_ID,
        memo_program: MEMO_PROGRAM_ID,
        token_2022_program: spl_token_2022_interface::ID,
        token_program: spl_token_2022_interface::ID,
    }
    .to_account_metas(None);

    // The mock Meteora program expects its reserve authority as the first
    // remaining account after the 16 fixed Swap2 accounts.
    liquidate_accounts.push(
        anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
            mock_reserve_authority,
            false,
        ),
    );

    let instruction = anchor_lang::solana_program::instruction::Instruction::new_with_bytes(
        program_id,
        &liquidate_data,
        liquidate_accounts,
    );
    // ---------------------------------------------------------
    // 13. Execute the actual Anchor instruction.
    //
    // Success means:
    //
    // Anchor account validation
    //      ↓
    // liquidation health validation
    //      ↓
    // liquidate handler
    //      ↓
    // invoke_signed()
    //      ↓
    // mock Meteora program
    //      ↓
    // Swap2 discriminator + account layout accepted
    // ---------------------------------------------------------

    let result: TransactionResult = send_transaction(&mut svm, instruction, &payer, &[&liquidator]);

    assert!(
        result.is_ok(),
        "liquidate CPI transaction failed: {:?}",
        result.err()
    );

    let debt_vault_after = common::token_2022_account_amount(
        &svm.get_account(&debt_vault)
            .expect("debt vault should exist")
            .data,
    );

    assert_eq!(
        debt_vault_after - debt_vault_before,
        2_500_000,
        "Meteora mock should deliver 2.5 USDC to the debt vault"
    );

    let position_account = svm
        .get_account(&position)
        .expect("position account should exist after liquidation");

    let position_state = Position::try_deserialize(&mut &position_account.data[..])
        .expect("failed to deserialize Position");

    assert_eq!(
        position_state.collateral_amount, 7_500_000_000,
        "liquidation should reduce collateral by 2.5 tokens"
    );

    assert_eq!(
        position_state.debt_amount, 3_500_000,
        "liquidation should reduce debt by 2.5 USDC"
    );
}
