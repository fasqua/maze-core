//! Error types for Maze Core
//!
//! Chain-agnostic errors only. Chain-specific errors belong in their
//! respective adapter crates (e.g. maze-solana).

use thiserror::Error;

#[derive(Debug, Error)]
pub enum MazeError {
    #[error("insufficient funds: required {required}, available {available}")]
    InsufficientFunds { required: u64, available: u64 },

    #[error("invalid parameters: {0}")]
    InvalidParameters(String),

    #[error("address generation failed: {0}")]
    AddressGeneration(String),

    #[error("encryption failed: {0}")]
    Encryption(String),
}

pub type Result<T> = std::result::Result<T, MazeError>;
