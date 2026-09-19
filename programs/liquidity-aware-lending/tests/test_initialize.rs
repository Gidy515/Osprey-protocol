/*use {
    anchor_lang::{
        prelude::Pubkey,
        solana_program::{instruction::Instruction, system_program},
        AccountDeserialize, InstructionData, ToAccountMetas,
    },
    litesvm::LiteSVM,
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
};

#[test]
fn test_initialize() {
    let program_id = liquidity_aware_lending::id();
    let payer = Keypair::new();
    let counter = Pubkey::find_program_address(
        &[liquidity_aware_lending::constants::COUNTER_SEED],
        &program_id,
    )
    .0;
    let mut svm = LiteSVM::new();
    let bytes = include_bytes!(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/../deploy/liquidity_aware_lending.so"
    ));
    svm.add_program(program_id, bytes).unwrap();
    svm.airdrop(&payer.pubkey(), 1_000_000_000).unwrap();

    let instruction = Instruction::new_with_bytes(
        program_id,
        &liquidity_aware_lending::instruction::Initialize {}.data(),
        liquidity_aware_lending::accounts::Initialize {
            payer: payer.pubkey(),
            counter,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[instruction], Some(&payer.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[&payer]).unwrap();

    let res = svm.send_transaction(tx);
    assert!(res.is_ok());

    let counter_account = svm.get_account(&counter).unwrap();
    let mut data: &[u8] = &counter_account.data;
    let counter_state = liquidity_aware_lending::state::Counter::try_deserialize(&mut data).unwrap();
    assert_eq!(counter_state.count, 0);
    assert_eq!(counter_state.authority, payer.pubkey());

    let instruction = Instruction::new_with_bytes(
        program_id,
        &liquidity_aware_lending::instruction::Increment {}.data(),
        liquidity_aware_lending::accounts::Increment {
            counter,
            authority: payer.pubkey(),
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[instruction], Some(&payer.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &[&payer]).unwrap();

    let res = svm.send_transaction(tx);
    assert!(res.is_ok());

    let counter_account = svm.get_account(&counter).unwrap();
    let mut data: &[u8] = &counter_account.data;
    let counter_state = liquidity_aware_lending::state::Counter::try_deserialize(&mut data).unwrap();
    assert_eq!(counter_state.count, 1);
    assert_eq!(counter_state.authority, payer.pubkey());
}*/
use anchor_lang::{prelude::Pubkey, AccountDeserialize, InstructionData, ToAccountMetas};

use litesvm::LiteSVM;
use solana_keypair::Keypair;
use solana_message::{Message, VersionedMessage};
use solana_signer::Signer;
use solana_transaction::versioned::VersionedTransaction;

use liquidity_aware_lending::{
    accounts::Initialize, constants::*, instruction::Initialize as InitializeInstruction,
    instructions::initialize::InitializeMarketParams, state::MarketConfig,
};

#[test]
fn test_initialize_market() {
    let program_id = liquidity_aware_lending::id();

    let payer = Keypair::new();

    let collateral_mint = Pubkey::new_unique();
    let debt_mint = Pubkey::new_unique();
    let dlmm_pool = Pubkey::new_unique();

    let (market, expected_bump) =
        Pubkey::find_program_address(&[MARKET_SEED, collateral_mint.as_ref()], &program_id);

    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/../deploy/liquidity_aware_lending.so"
    ));

    svm.add_program(program_id, program_bytes).unwrap();

    svm.airdrop(&payer.pubkey(), 1_000_000_000).unwrap();

    let params = InitializeMarketParams {
        collateral_mint,
        debt_mint,
        dlmm_pool,
    };

    let instruction = anchor_lang::solana_program::instruction::Instruction::new_with_bytes(
        program_id,
        &InitializeInstruction {
            params: params.clone(),
        }
        .data(),
        Initialize {
            market,
            authority: payer.pubkey(),
            system_program: anchor_lang::solana_program::system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let message = Message::new_with_blockhash(&[instruction], Some(&payer.pubkey()), &blockhash);

    let transaction =
        VersionedTransaction::try_new(VersionedMessage::Legacy(message), &[&payer]).unwrap();

    let result = svm.send_transaction(transaction);

    assert!(
        result.is_ok(),
        "initialize transaction failed: {:?}",
        result.err()
    );

    let market_account = svm
        .get_account(&market)
        .expect("market account should exist");

    let mut data: &[u8] = &market_account.data;

    let market_state =
        MarketConfig::try_deserialize(&mut data).expect("failed to deserialize MarketConfig");

    assert_eq!(market_state.collateral_mint, collateral_mint);
    assert_eq!(market_state.debt_mint, debt_mint);
    assert_eq!(market_state.dlmm_pool, dlmm_pool);
    assert_eq!(market_state.authority, payer.pubkey());

    assert_eq!(market_state.max_ltv_bps, MAX_LTV_BPS);
    assert_eq!(market_state.min_ltv_bps, MIN_LTV_BPS);
    assert_eq!(
        market_state.liquidation_threshold_bps,
        LIQUIDATION_THRESHOLD_BPS
    );
    assert_eq!(market_state.liquidation_bonus_bps, LIQUIDATION_BONUS_BPS);
    assert_eq!(market_state.max_liquidation_bps, MAX_LIQUIDATION_BPS);

    assert_eq!(
        market_state.reference_liquidation_size_usdc,
        REFERENCE_LIQUIDATION_SIZE_USDC
    );

    assert_eq!(
        market_state.issuer_risk_ceiling_bps,
        ISSUER_RISK_CEILING_BPS
    );

    assert_eq!(market_state.transfer_fee_bps, PRESTOCKS_TRANSFER_FEE_BPS);

    assert_eq!(market_state.bump, expected_bump);
}
