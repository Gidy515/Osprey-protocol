#![no_std]

extern crate alloc;

use alloc::vec;

use solana_account_info::AccountInfo;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::program::invoke_signed;
use solana_program_entrypoint::{entrypoint, ProgramResult};
use solana_pubkey::Pubkey;

const SWAP2_DISCRIMINATOR: [u8; 8] = [65, 75, 63, 76, 235, 91, 91, 136];

// -----------------------------------------------------------------------------
// Swap2 fixed-account positions.
//
// These correspond to the account ordering constructed by
// integrations/meteora.rs in the lending program.
// -----------------------------------------------------------------------------

const RESERVE_Y_INDEX: usize = 3;
const USER_TOKEN_OUT_INDEX: usize = 5;
const TOKEN_Y_PROGRAM_INDEX: usize = 12;

// -----------------------------------------------------------------------------
// Remaining account supplied by the lending program.
//
// The mock needs this PDA because it owns the mock USDC reserve token
// account and must sign the nested Token-2022 transfer CPI.
// -----------------------------------------------------------------------------

const RESERVE_AUTHORITY_INDEX: usize = 16;

// PDA controlling the mock USDC reserve.
const RESERVE_AUTHORITY_SEED: &[u8] = b"mock-reserve-authority";

// SPL Token / Token-2022 `Transfer` instruction discriminator.
const TRANSFER_DISCRIMINATOR: u8 = 3;

// -----------------------------------------------------------------------------
// Mock exchange rate.
//
// The collateral mint has 9 decimals.
// The debt mint has 6 decimals.
//
// Therefore:
//
//     1 collateral = 1 USDC
//
// In raw token units:
//
//     1_000_000_000 collateral units
//         ->
//     1_000_000 USDC units
//
// Hence the conversion factor is 1,000.
// -----------------------------------------------------------------------------

const COLLATERAL_TO_USDC_SCALE: u64 = 1_000;

entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    // -------------------------------------------------------------------------
    // 1. Validate instruction data length.
    //
    // Swap2 data expected by our mock:
    //
    // bytes 0..8   = discriminator
    // bytes 8..16  = amount_in
    // bytes 16..24 = min_amount_out
    // -------------------------------------------------------------------------

    if instruction_data.len() < 24 {
        return Err(solana_program_error::ProgramError::InvalidInstructionData);
    }

    // -------------------------------------------------------------------------
    // 2. Validate that this is the Swap2 instruction.
    // -------------------------------------------------------------------------

    if instruction_data[..8] != SWAP2_DISCRIMINATOR {
        return Err(solana_program_error::ProgramError::InvalidInstructionData);
    }

    // -------------------------------------------------------------------------
    // 3. We expect:
    //
    //     16 fixed Swap2 accounts
    //     + 1 remaining account containing the reserve authority PDA
    //
    // Therefore at least 17 accounts are required.
    // -------------------------------------------------------------------------

    if accounts.len() < 17 {
        return Err(solana_program_error::ProgramError::NotEnoughAccountKeys);
    }

    // -------------------------------------------------------------------------
    // 4. Decode amount_in.
    // -------------------------------------------------------------------------

    let amount_in = u64::from_le_bytes(
        instruction_data[8..16]
            .try_into()
            .map_err(|_| solana_program_error::ProgramError::InvalidInstructionData)?,
    );

    // -------------------------------------------------------------------------
    // 5. Decode min_amount_out.
    // -------------------------------------------------------------------------

    let min_amount_out = u64::from_le_bytes(
        instruction_data[16..24]
            .try_into()
            .map_err(|_| solana_program_error::ProgramError::InvalidInstructionData)?,
    );

    if amount_in == 0 {
        return Err(solana_program_error::ProgramError::InvalidArgument);
    }

    // -------------------------------------------------------------------------
    // 6. Calculate deterministic mock swap output.
    //
    // Example:
    //
    //     amount_in = 1_000_000_000
    //
    //     amount_out =
    //         1_000_000_000 / 1_000
    //         = 1_000_000
    //
    // which represents 1 USDC.
    // -------------------------------------------------------------------------

    let amount_out = amount_in
        .checked_div(COLLATERAL_TO_USDC_SCALE)
        .ok_or(solana_program_error::ProgramError::ArithmeticOverflow)?;

    if amount_out == 0 || amount_out < min_amount_out {
        return Err(solana_program_error::ProgramError::InsufficientFunds);
    }

    // -------------------------------------------------------------------------
    // 7. Read the fixed Swap2 accounts.
    // -------------------------------------------------------------------------

    let reserve_y = &accounts[RESERVE_Y_INDEX];

    let user_token_out = &accounts[USER_TOKEN_OUT_INDEX];

    let token_y_program = &accounts[TOKEN_Y_PROGRAM_INDEX];

    // -------------------------------------------------------------------------
    // 8. Read the reserve authority supplied through remaining_accounts.
    //
    // The lending program appends ctx.remaining_accounts after the 16 fixed
    // Meteora accounts. Our test will therefore provide the PDA as account 16.
    // -------------------------------------------------------------------------

    let reserve_authority_account = &accounts[RESERVE_AUTHORITY_INDEX];

    // -------------------------------------------------------------------------
    // 9. Derive the reserve authority PDA ourselves.
    //
    // This lets us verify that the supplied remaining account is actually
    // the PDA controlled by this mock Meteora program.
    // -------------------------------------------------------------------------

    let (reserve_authority, reserve_bump) =
        Pubkey::find_program_address(&[RESERVE_AUTHORITY_SEED], program_id);

    if reserve_authority_account.key != &reserve_authority {
        return Err(solana_program_error::ProgramError::InvalidArgument);
    }

    // -------------------------------------------------------------------------
    // 10. Build the Token-2022 `Transfer` instruction.
    //
    // Basic SPL Token / Token-2022 Transfer accounts are:
    //
    //     0. source
    //     1. destination
    //     2. authority
    //
    // The mint account is NOT required for the basic Transfer instruction.
    // -------------------------------------------------------------------------

    let transfer_data = {
        let mut data = [0u8; 9];

        // Transfer instruction discriminator.
        data[0] = TRANSFER_DISCRIMINATOR;

        // Amount being transferred.
        data[1..9].copy_from_slice(&amount_out.to_le_bytes());

        data
    };

    let transfer_accounts = vec![
        AccountMeta::new(*reserve_y.key, false),
        AccountMeta::new(*user_token_out.key, false),
        AccountMeta::new_readonly(reserve_authority, true),
    ];

    let transfer_instruction = Instruction {
        program_id: *token_y_program.key,
        accounts: transfer_accounts,
        data: transfer_data.to_vec(),
    };

    // -------------------------------------------------------------------------
    // 11. Prepare PDA signer seeds.
    //
    // The reserve authority PDA is owned by this mock Meteora program.
    // Therefore this program can sign for it using invoke_signed.
    // -------------------------------------------------------------------------

    let reserve_bump_seed = [reserve_bump];

    let signer_seeds: &[&[u8]] = &[RESERVE_AUTHORITY_SEED, &reserve_bump_seed];

    // -------------------------------------------------------------------------
    // 12. Execute the nested Token-2022 transfer CPI.
    //
    // This is the important part:
    //
    // mock Meteora
    //      |
    //      | CPI
    //      v
    // Token-2022
    //      |
    //      | transfer USDC
    //      v
    // reserve_y -> debt_vault
    //
    // The mock cannot directly mutate reserve_y/debt_vault because those
    // accounts are owned by Token-2022.
    // -------------------------------------------------------------------------

    invoke_signed(
        &transfer_instruction,
        &[
            reserve_y.clone(),
            user_token_out.clone(),
            reserve_authority_account.clone(),
            token_y_program.clone(),
        ],
        &[signer_seeds],
    )?;

    Ok(())
}
