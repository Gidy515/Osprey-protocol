pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("6HxjRZGNXgKE21WRsd5RbwXJGmNCcQrpEJ1uzBnBJ8M6");

#[program]
pub mod liquidity_aware_lending {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        params: InitializeMarketParams,
    ) -> Result<()> {
        instructions::initialize::handle_initialize(ctx, params)
    }
}
