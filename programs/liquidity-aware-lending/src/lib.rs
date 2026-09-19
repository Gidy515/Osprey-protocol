pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("jAUJs14M4WiAvrgRgZiKQ2nunkWMrqqM7sGZotmWwss");

#[program]
pub mod liquidity_aware_lending {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, params: InitializeMarketParams) -> Result<()> {
        instructions::initialize::handle_initialize(ctx, params)
    }

    pub fn deposit_collateral(ctx: Context<DepositCollateral>, amount: u64) -> Result<()> {
        instructions::deposit_collateral::handle_deposit_collateral(ctx, amount)
    }

    pub fn borrow(ctx: Context<Borrow>, amount: u64) -> Result<()> {
        instructions::borrow::handle_borrow(ctx, amount)
    }
}
