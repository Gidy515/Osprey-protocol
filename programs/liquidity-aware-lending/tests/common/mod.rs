use anchor_lang::{prelude::Pubkey, InstructionData, ToAccountMetas};
use anchor_spl::associated_token::get_associated_token_address_with_program_id;

use litesvm::LiteSVM;

use solana_keypair::Keypair;
use solana_message::{Message, VersionedMessage};
use solana_signer::Signer;
use solana_transaction::versioned::VersionedTransaction;

use spl_associated_token_account_interface::instruction::create_associated_token_account;

use spl_token_2022_interface::{
    extension::transfer_fee::instruction::initialize_transfer_fee_config, extension::ExtensionType,
    instruction as token_instruction, state::Mint, ID as TOKEN_2022_PROGRAM_ID,
};

use liquidity_aware_lending::{
    constants::*, instruction::Initialize as InitializeInstruction,
    instructions::initialize::InitializeMarketParams,
};

pub fn token_2022_account_amount(account_data: &[u8]) -> u64 {
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

pub fn send_transaction(
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

pub fn create_token_2022_mint(
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

pub fn create_token_2022_transfer_fee_mint(
    svm: &mut LiteSVM,
    payer: &Keypair,
    mint: &Keypair,
    decimals: u8,
    mint_authority: &Pubkey,
    transfer_fee_basis_points: u16,
    maximum_fee: u64,
) {
    let extension_types = &[ExtensionType::TransferFeeConfig];

    let mint_space = ExtensionType::try_calculate_account_len::<Mint>(extension_types)
        .expect("failed to calculate Token-2022 mint size");

    let mint_rent = svm.minimum_balance_for_rent_exemption(mint_space);

    let create_mint_account = anchor_lang::solana_program::system_instruction::create_account(
        &payer.pubkey(),
        &mint.pubkey(),
        mint_rent,
        mint_space as u64,
        &TOKEN_2022_PROGRAM_ID,
    );

    let initialize_transfer_fee_config = initialize_transfer_fee_config(
        &TOKEN_2022_PROGRAM_ID,
        &mint.pubkey(),
        None,
        None,
        transfer_fee_basis_points,
        maximum_fee,
    )
    .expect("failed to create transfer fee config instruction");

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
        &[
            create_mint_account,
            initialize_transfer_fee_config,
            initialize_mint,
        ],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let transaction =
        VersionedTransaction::try_new(VersionedMessage::Legacy(message), &[payer, mint])
            .expect("failed to construct transfer-fee mint transaction");

    svm.send_transaction(transaction)
        .expect("failed to create Token-2022 transfer-fee mint");
}

pub fn create_token_2022_ata(svm: &mut LiteSVM, payer: &Keypair, owner: &Pubkey, mint: &Pubkey) {
    let instruction =
        create_associated_token_account(&payer.pubkey(), owner, mint, &TOKEN_2022_PROGRAM_ID);

    let blockhash = svm.latest_blockhash();

    let message = Message::new_with_blockhash(&[instruction], Some(&payer.pubkey()), &blockhash);

    let transaction = VersionedTransaction::try_new(VersionedMessage::Legacy(message), &[payer])
        .expect("failed to construct ATA transaction");

    svm.send_transaction(transaction)
        .expect("failed to create Token-2022 ATA");
}

pub fn mint_token_2022(
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

    let transaction = if payer.pubkey() == mint_authority.pubkey() {
        VersionedTransaction::try_new(VersionedMessage::Legacy(message), &[payer])
    } else {
        VersionedTransaction::try_new(VersionedMessage::Legacy(message), &[payer, mint_authority])
    }
    .expect("failed to construct mint transaction");

    svm.send_transaction(transaction)
        .expect("failed to mint Token-2022 tokens");
}

pub fn initialize_market(
    svm: &mut LiteSVM,
    program_id: Pubkey,
    payer: &Keypair,
    collateral_mint: Pubkey,
    debt_mint: Pubkey,
    dlmm_pool: Pubkey,
) -> Pubkey {
    let (market, _) =
        Pubkey::find_program_address(&[MARKET_SEED, collateral_mint.as_ref()], &program_id);

    let params = InitializeMarketParams {
        collateral_mint,
        debt_mint,
        dlmm_pool,
        // MVP test price: 1 collateral token = 1 USDC.
        // Collateral uses 9 decimals, USDC uses 6 decimals.
        collateral_price_usdc: 1_000_000,
    };

    let instruction = anchor_lang::solana_program::instruction::Instruction::new_with_bytes(
        program_id,
        &InitializeInstruction { params }.data(),
        liquidity_aware_lending::accounts::Initialize {
            market,
            authority: payer.pubkey(),
            system_program: anchor_lang::solana_program::system_program::ID,
        }
        .to_account_metas(None),
    );

    let result = send_transaction(svm, instruction, payer, &[]);

    assert!(
        result.is_ok(),
        "initialize transaction failed: {:?}",
        result.err()
    );

    market
}

pub fn setup_lending_program() -> (LiteSVM, Keypair) {
    let program_id = liquidity_aware_lending::id();
    let payer = Keypair::new();
    let mut svm = LiteSVM::new();

    let program_bytes = include_bytes!(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/../deploy/liquidity_aware_lending.so"
    ));

    svm.add_program(program_id, program_bytes)
        .expect("failed to add lending program");

    let meteora_program_bytes = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../target/deploy/mock_meteora.so"
    ));

    svm.add_program(
        liquidity_aware_lending::integrations::meteora::METEORA_DLMM_PROGRAM_ID,
        meteora_program_bytes,
    )
    .expect("failed to add mock Meteora program");

    svm.airdrop(&payer.pubkey(), 5_000_000_000)
        .expect("failed to airdrop payer");

    (svm, payer)
}

pub fn associated_token_address(owner: &Pubkey, mint: &Pubkey) -> Pubkey {
    get_associated_token_address_with_program_id(owner, mint, &TOKEN_2022_PROGRAM_ID)
}

pub fn initialize_liquidity_risk(
    svm: &mut litesvm::LiteSVM,
    program_id: anchor_lang::prelude::Pubkey,
    payer: &solana_keypair::Keypair,
    market: anchor_lang::prelude::Pubkey,
    quote_collateral_in: u64,
    quote_usdc_out: u64,
) -> anchor_lang::prelude::Pubkey {
    use anchor_lang::{InstructionData, ToAccountMetas};
    use solana_signer::Signer;

    use liquidity_aware_lending::{
        accounts::UpdateLiquidityRisk, constants::RISK_SNAPSHOT_SEED,
        instruction::UpdateLiquidityRisk as UpdateLiquidityRiskInstruction,
    };

    let (risk_snapshot, _) = anchor_lang::prelude::Pubkey::find_program_address(
        &[RISK_SNAPSHOT_SEED, market.as_ref()],
        &program_id,
    );

    let accounts = UpdateLiquidityRisk {
        authority: payer.pubkey(),
        market,
        risk_snapshot,
        system_program: anchor_lang::solana_program::system_program::ID,
    };

    let data = UpdateLiquidityRiskInstruction {
        quote_collateral_in,
        quote_usdc_out,
    };

    let instruction = anchor_lang::solana_program::instruction::Instruction::new_with_bytes(
        program_id,
        &data.data(),
        accounts.to_account_metas(None),
    );

    let result = send_transaction(svm, instruction, payer, &[]);

    assert!(
        result.is_ok(),
        "risk snapshot initialization failed: {:?}",
        result.err()
    );

    risk_snapshot
}
