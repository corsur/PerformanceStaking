use anchor_lang::prelude::*;

#[account]
pub struct Five90State {
    pub admin: Pubkey, // Admin key for controlling stake allocations
    pub epoch: u64,    // Last processed epoch
}