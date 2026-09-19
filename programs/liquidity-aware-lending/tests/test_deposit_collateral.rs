use anchor_lang::{prelude::Pubkey, AccountDeserialize, InstructionData, ToAccountMetas};

use anchor_spl::associated_token::get_associated_token_address_with_program_id;

use litesvm::LiteSVM;

use solana_keypair::Keypair;
use solana_message::{Message, VersionedMessage};
use solana_signer::Signer;
use solana_transaction::versioned::VersionedTransaction;

use spl_associated_token_account_interface::{
    instruction::create_associated_token_account, program::ID as ASSOCIATED_TOKEN_PROGRAM_ID,
};

use spl_token_2022_interface::{instruction as token_instruction, ID as TOKEN_2022_PROGRAM_ID};

use liquidity_aware_lending::{
    accounts::DepositCollateral,
    constants::*,
    instruction::DepositCollateral as DepositCollateralInstruction,
    state::{MarketConfig, Position},
};

fn token_2022_account_amount(account_data: &[u8]) -> u64 {
    assert!(
        account_data.len() >= 72,
        "Token-2022 account data is too short"
    );

    u64::from_le_bytes(
        account_data[64..72]
            .try_into()
            .expect("invalid token amount bytes"),
    )
}

/// Send a single instruction using the same transaction construction
/// pattern used by the working test_initialize.rs.
fn send_transaction(
    svm: &mut LiteSVM,
    instruction: anchor_lang::solana_program::instruction::Instruction,
    payer: &Keypair,
    additional_signers: &[&Keypair],
) -> litesvm::types::TransactionResult {
    let blockhash = svm.latest_blockhash();

    let message = Message::new_with_blockhash(&[instruction], Some(&payer.pubkey()), &blockhash);

    let mut signers: Vec<&Keypair> = Vec::with_capacity(1 + additional_signers.len());

    signers.push(payer);
    signers.extend_from_slice(additional_signers);

    let transaction = VersionedTransaction::try_new(VersionedMessage::Legacy(message), &signers)
        .expect("failed to construct transaction");

    svm.send_transaction(transaction)
}

/// Create a Token-2022 mint.
fn create_token_2022_mint(
    svm: &mut LiteSVM,
    payer: &Keypair,
    mint: &Keypair,
    decimals: u8,
    mint_authority: &Pubkey,
) {
    let mint_rent = svm.minimum_balance_for_rent_exemption(82);

    let create_mint_account = anchor_lang::solana_program::system_instruction::create_account(
        &payer.pubkey(),
        &mint.pubkey(),
        mint_rent,
        82,
        &TOKEN_2022_PROGRAM_ID,
    );

    let initialize_mint = token_instruction::initialize_mint2(
        &TOKEN_2022_PROGRAM_ID,
        &mint.pubkey(),
        mint_authority,
        None,
        decimals,
    )
    .expect("failed to create initialize_mint2 instruction");

    let blockhash = svm.latest_blockhash();

    let message = Message::new_with_blockhash(
        &[create_mint_account, initialize_mint],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let transaction =
        VersionedTransaction::try_new(VersionedMessage::Legacy(message), &[payer, mint])
            .expect("failed to construct mint transaction");

    svm.send_transaction(transaction)
        .expect("failed to create Token-2022 mint");
}

/// Create a Token-2022 associated token account for a wallet.
fn create_token_2022_ata(svm: &mut LiteSVM, payer: &Keypair, owner: &Pubkey, mint: &Pubkey) {
    let instruction =
        create_associated_token_account(&payer.pubkey(), owner, mint, &TOKEN_2022_PROGRAM_ID);

    let blockhash = svm.latest_blockhash();

    let message = Message::new_with_blockhash(&[instruction], Some(&payer.pubkey()), &blockhash);

    let transaction = VersionedTransaction::try_new(VersionedMessage::Legacy(message), &[payer])
        .expect("failed to construct ATA transaction");

    svm.send_transaction(transaction)
        .expect("failed to create Token-2022 ATA");
}

/// Mint Token-2022 tokens to a token account.
fn mint_token_2022(
    svm: &mut LiteSVM,
    payer: &Keypair,
    mint: &Pubkey,
    destination: &Pubkey,
    mint_authority: &Keypair,
    amount: u64,
) {
    let instruction = token_instruction::mint_to(
        &TOKEN_2022_PROGRAM_ID,
        mint,
        destination,
        &mint_authority.pubkey(),
        &[],
        amount,
    )
    .expect("failed to create mint_to instruction");

    let blockhash = svm.latest_blockhash();

    let message = Message::new_with_blockhash(&[instruction], Some(&payer.pubkey()), &blockhash);

    let transaction =
        VersionedTransaction::try_new(VersionedMessage::Legacy(message), &[payer, mint_authority])
            .expect("failed to construct mint transaction");

    svm.send_transaction(transaction)
        .expect("failed to mint Token-2022 tokens");
}

#[test]
fn test_deposit_collateral() {
    let program_id = liquidity_aware_lending::id();

    let payer = Keypair::new();

    // ---------------------------------------------------------------
    // LiteSVM setup
    // ---------------------------------------------------------------

    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/../deploy/liquidity_aware_lending.so"
    ));

    svm.add_program(program_id, program_bytes).unwrap();

    svm.airdrop(&payer.pubkey(), 5_000_000_000)
        .expect("failed to airdrop payer");

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

    let dlmm_pool = Pubkey::new_unique();

    let (market, expected_market_bump) =
        Pubkey::find_program_address(&[MARKET_SEED, collateral_mint_pubkey.as_ref()], &program_id);

    let initialize_params =
        liquidity_aware_lending::instructions::initialize::InitializeMarketParams {
            collateral_mint: collateral_mint_pubkey,
            debt_mint: debt_mint_pubkey,
            dlmm_pool,
        };

    let initialize_instruction =
        anchor_lang::solana_program::instruction::Instruction::new_with_bytes(
            program_id,
            &liquidity_aware_lending::instruction::Initialize {
                params: initialize_params,
            }
            .data(),
            liquidity_aware_lending::accounts::Initialize {
                market,
                authority: payer.pubkey(),
                system_program: anchor_lang::solana_program::system_program::ID,
            }
            .to_account_metas(None),
        );

    let initialize_result = send_transaction(&mut svm, initialize_instruction, &payer, &[]);

    assert!(
        initialize_result.is_ok(),
        "initialize transaction failed: {:?}",
        initialize_result.err()
    );

    // ---------------------------------------------------------------
    // Verify MarketConfig
    // ---------------------------------------------------------------

    let market_account = svm
        .get_account(&market)
        .expect("market account should exist");

    let mut market_data: &[u8] = &market_account.data;

    let market_state = MarketConfig::try_deserialize(&mut market_data)
        .expect("failed to deserialize MarketConfig");

    assert_eq!(market_state.collateral_mint, collateral_mint_pubkey);
    assert_eq!(market_state.debt_mint, debt_mint_pubkey);
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
    assert_eq!(market_state.bump, expected_market_bump);

    // ---------------------------------------------------------------
    // Derive user's Token-2022 ATA
    // ---------------------------------------------------------------

    let user_collateral_account = get_associated_token_address_with_program_id(
        &payer.pubkey(),
        &collateral_mint_pubkey,
        &TOKEN_2022_PROGRAM_ID,
    );

    // ---------------------------------------------------------------
    // Create user's Token-2022 ATA
    // ---------------------------------------------------------------

    create_token_2022_ata(&mut svm, &payer, &payer.pubkey(), &collateral_mint_pubkey);

    // ---------------------------------------------------------------
    // Derive vault authority PDA
    // ---------------------------------------------------------------

    let (vault_authority, _vault_bump) =
        Pubkey::find_program_address(&[VAULT_SEED, market.as_ref()], &program_id);

    // ---------------------------------------------------------------
    // Derive vault Token-2022 ATA
    //
    // We intentionally DO NOT create it here.
    //
    // deposit_collateral has:
    //
    //     init_if_needed
    //
    // so the lending program itself should create it.
    // ---------------------------------------------------------------

    let vault_collateral_account = get_associated_token_address_with_program_id(
        &vault_authority,
        &collateral_mint_pubkey,
        &TOKEN_2022_PROGRAM_ID,
    );

    // ---------------------------------------------------------------
    // Mint collateral to user
    // ---------------------------------------------------------------

    let initial_collateral_amount: u64 = 1_000_000_000; // 1 token

    mint_token_2022(
        &mut svm,
        &payer,
        &collateral_mint_pubkey,
        &user_collateral_account,
        &collateral_mint_authority,
        initial_collateral_amount,
    );

    // ---------------------------------------------------------------
    // Verify user's initial Token-2022 balance
    // ---------------------------------------------------------------

    let user_account_before = svm
        .get_account(&user_collateral_account)
        .expect("user collateral ATA should exist");

    let user_balance_before = token_2022_account_amount(&user_account_before.data);

    assert_eq!(
        user_balance_before, initial_collateral_amount,
        "user should own the minted collateral"
    );

    // ---------------------------------------------------------------
    // Derive Position PDA
    // ---------------------------------------------------------------

    let (position, _position_bump) = Pubkey::find_program_address(
        &[POSITION_SEED, market.as_ref(), payer.pubkey().as_ref()],
        &program_id,
    );

    // ---------------------------------------------------------------
    // Deposit collateral
    // ---------------------------------------------------------------

    let deposit_amount: u64 = 500_000_000; // 0.5 token

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

    let deposit_result = send_transaction(&mut svm, deposit_instruction, &payer, &[]);

    assert!(
        deposit_result.is_ok(),
        "deposit transaction failed: {:?}",
        deposit_result.err()
    );

    // ---------------------------------------------------------------
    // Verify Position
    // ---------------------------------------------------------------

    let position_account = svm
        .get_account(&position)
        .expect("position account should exist");

    let mut position_data: &[u8] = &position_account.data;

    let position_state =
        Position::try_deserialize(&mut position_data).expect("failed to deserialize Position");

    assert_eq!(position_state.owner, payer.pubkey());
    assert_eq!(position_state.market, market);

    /*
     * The test mint has no transfer fee.
     *
     * Therefore:
     *
     * amount requested = amount received
     *
     * The production implementation still measures the vault's
     * before/after balance, which is important for real PreStocks
     * Token-2022 transfer fees.
     */
    assert_eq!(
        position_state.collateral_amount, deposit_amount,
        "position collateral should equal the amount actually received"
    );

    // ---------------------------------------------------------------
    // Verify vault balance
    // ---------------------------------------------------------------

    let vault_account = svm
        .get_account(&vault_collateral_account)
        .expect("vault collateral ATA should exist");

    let vault_balance = token_2022_account_amount(&vault_account.data);

    assert_eq!(
        vault_balance, deposit_amount,
        "vault should contain the deposited collateral"
    );

    // ---------------------------------------------------------------
    // Verify user balance decreased
    // ---------------------------------------------------------------

    let user_account_after = svm
        .get_account(&user_collateral_account)
        .expect("user collateral ATA should still exist");

    let user_balance_after = token_2022_account_amount(&user_account_after.data);

    assert_eq!(
        user_balance_after,
        initial_collateral_amount - deposit_amount,
        "user collateral balance should decrease by the deposited amount"
    );
}
