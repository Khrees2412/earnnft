use anchor_lang::prelude::*;

#[error_code]
pub enum CustomError {
    #[msg("Invalid basis points")]
    InvalidBps,
    #[msg("Invalid unlock timestamp")]
    InvalidUnlockTs,
    #[msg("Current time is before unlock")]
    NotYetUnlocked,
    #[msg("State does not match collection")]
    StateCollectionMismatch,
    #[msg("Update authority mismatch")]
    BadAuthority,
    #[msg("Invalid Metaplex Core program ID")]
    InvalidMplCoreProgram,
}
