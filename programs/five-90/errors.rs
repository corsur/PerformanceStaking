use anchor_lang::prelude::*;

#[error_code]
pub enum Five90Error {
    #[msg("The provided epoch is invalid.")]
    InvalidEpoch,
}