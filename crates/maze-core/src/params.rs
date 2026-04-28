//! Maze routing parameters
//!
//! Defines the configuration for maze topology generation.
//! All parameters are chain-agnostic. Chain-specific defaults
//! (e.g. transaction fee amounts) are set by adapter crates.

use serde::{Deserialize, Serialize};

/// Default hop count
pub const DEFAULT_HOPS: u8 = 7;

/// Minimum hops in maze
pub const MIN_HOPS: u8 = 5;

/// Maximum hops in maze
pub const MAX_HOPS: u8 = 10;

/// Default amount noise percentage
pub const DEFAULT_AMOUNT_NOISE: f64 = 0.5;

/// Default base delay in milliseconds
pub const DEFAULT_DELAY_MS: u64 = 500;

/// Maze generation parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MazeParameters {
    /// Random seed for deterministic generation (256-bit)
    pub seed: [u8; 32],
    /// Fibonacci offset for path variation (0-100)
    pub fib_offset: u8,
    /// Split ratio based on golden ratio variant (1.1-3.0)
    pub split_ratio: f64,
    /// Total number of hops/levels in maze
    pub hop_count: u8,
    /// Merge strategy
    pub merge_strategy: MergeStrategy,
    /// Delay pattern between transactions
    pub delay_pattern: DelayPattern,
    /// Amount variation percentage (0.01% - 1%)
    pub amount_noise: f64,
    /// Base delay in milliseconds (0-5000)
    pub delay_ms: u64,
    /// Delay scope: per node or per level
    pub delay_scope: DelayScope,
    /// Transaction fee per TX in base units (e.g. lamports for Solana)
    pub tx_fee: u64,
    /// Pool address for privacy relay (optional)
    #[serde(default)]
    pub pool_address: Option<String>,
    /// Pool signing material bytes (optional, not serialized)
    #[serde(skip)]
    pub pool_signing_material: Option<Vec<u8>>,
}

/// Strategy for when to merge nodes in the maze topology.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum MergeStrategy {
    /// Merge in first half of hops, split in second half
    Early,
    /// Split in first half of hops, merge in second half
    Late,
    /// Split at edges, merge in middle
    Middle,
    /// Deterministic random based on seed
    Random,
    /// Based on fibonacci sequence parity
    Fibonacci,
}

/// Pattern for delays between transactions.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum DelayPattern {
    /// No delay
    None,
    /// Linear increase
    Linear,
    /// Exponential increase
    Exponential,
    /// Deterministic random delays
    Random,
    /// Fibonacci-based delays
    Fibonacci,
}

/// Scope of delay application.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum DelayScope {
    /// Apply delay per node
    Node,
    /// Apply delay per level
    Level,
}

impl Default for MazeParameters {
    fn default() -> Self {
        Self {
            seed: rand::random(),
            fib_offset: rand::random::<u8>() % 100,
            split_ratio: 1.618,
            hop_count: DEFAULT_HOPS,
            merge_strategy: MergeStrategy::Random,
            delay_pattern: DelayPattern::Random,
            amount_noise: DEFAULT_AMOUNT_NOISE,
            delay_ms: DEFAULT_DELAY_MS,
            delay_scope: DelayScope::Node,
            tx_fee: 5_000,
            pool_address: None,
            pool_signing_material: None,
        }
    }
}

impl MazeParameters {
    /// Generate parameters with randomized values.
    pub fn random() -> Self {
        let mut params = Self::default();
        params.seed = rand::random();
        params.fib_offset = rand::random::<u8>() % 100;
        params.split_ratio = 1.1 + (rand::random::<f64>() * 1.9);
        params.merge_strategy = match rand::random::<u8>() % 5 {
            0 => MergeStrategy::Early,
            1 => MergeStrategy::Late,
            2 => MergeStrategy::Middle,
            3 => MergeStrategy::Fibonacci,
            _ => MergeStrategy::Random,
        };
        params.delay_pattern = match rand::random::<u8>() % 5 {
            0 => DelayPattern::None,
            1 => DelayPattern::Linear,
            2 => DelayPattern::Exponential,
            3 => DelayPattern::Fibonacci,
            _ => DelayPattern::Random,
        };
        params
    }

    /// Validate parameters are within acceptable ranges.
    pub fn validate(&self) -> crate::error::Result<()> {
        if self.hop_count < MIN_HOPS || self.hop_count > MAX_HOPS {
            return Err(crate::error::MazeError::InvalidParameters(
                format!("hop_count must be between {} and {}, got {}", MIN_HOPS, MAX_HOPS, self.hop_count)
            ));
        }
        if self.split_ratio < 1.1 || self.split_ratio > 3.0 {
            return Err(crate::error::MazeError::InvalidParameters(
                format!("split_ratio must be between 1.1 and 3.0, got {}", self.split_ratio)
            ));
        }
        if self.amount_noise < 0.0 || self.amount_noise > 1.0 {
            return Err(crate::error::MazeError::InvalidParameters(
                format!("amount_noise must be between 0.0 and 1.0, got {}", self.amount_noise)
            ));
        }
        if self.tx_fee == 0 {
            return Err(crate::error::MazeError::InvalidParameters(
                "tx_fee must be greater than 0".to_string()
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_params() {
        let params = MazeParameters::default();
        assert_eq!(params.hop_count, DEFAULT_HOPS);
        assert_eq!(params.split_ratio, 1.618);
        assert_eq!(params.tx_fee, 5_000);
        assert!(params.pool_address.is_none());
        assert!(params.pool_signing_material.is_none());
    }

    #[test]
    fn test_random_params() {
        let a = MazeParameters::random();
        let b = MazeParameters::random();
        assert_ne!(a.seed, b.seed, "random params should have different seeds");
    }

    #[test]
    fn test_validate_valid() {
        let params = MazeParameters::default();
        assert!(params.validate().is_ok());
    }

    #[test]
    fn test_validate_hop_count_too_low() {
        let mut params = MazeParameters::default();
        params.hop_count = 2;
        assert!(params.validate().is_err());
    }

    #[test]
    fn test_validate_hop_count_too_high() {
        let mut params = MazeParameters::default();
        params.hop_count = 20;
        assert!(params.validate().is_err());
    }

    #[test]
    fn test_validate_split_ratio_out_of_range() {
        let mut params = MazeParameters::default();
        params.split_ratio = 0.5;
        assert!(params.validate().is_err());
    }

    #[test]
    fn test_validate_zero_tx_fee() {
        let mut params = MazeParameters::default();
        params.tx_fee = 0;
        assert!(params.validate().is_err());
    }
}
