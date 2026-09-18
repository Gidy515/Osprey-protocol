use anchor_lang::prelude::*;

pub const MARKET_SEED: &[u8] = b"market";
pub const POSITION_SEED: &[u8] = b"position";

pub const MAX_LTV_BPS: u16 = 6_500;
pub const MIN_LTV_BPS: u16 = 3_500;

pub const LIQUIDATION_THRESHOLD_BPS: u16 = 7_000;
pub const LIQUIDATION_BONUS_BPS: u16 = 500;
pub const MAX_LIQUIDATION_BPS: u16 = 2_500;

pub const ISSUER_RISK_CEILING_BPS: u16 = 6_000;

/// USDC has 6 decimals.
/// $5,000 = 5_000_000_000 base units.
pub const REFERENCE_LIQUIDATION_SIZE_USDC: u64 = 5_000_000_000;

pub const PRESTOCKS_TRANSFER_FEE_BPS: u16 = 50;