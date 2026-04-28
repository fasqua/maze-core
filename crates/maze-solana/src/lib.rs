//! Maze Solana: Solana adapter for maze-core
//!
//! Provides SolanaAddressGenerator that implements the AddressGenerator
//! trait using Solana keypairs.

use maze_core::AddressGenerator;
use maze_core::error::Result;
use solana_sdk::signature::{Keypair, Signer};

/// Solana address generator for maze routing.
///
/// Generates ephemeral Solana keypairs for use as intermediate
/// maze nodes. Each call produces a unique keypair that has
/// never existed before.
pub struct SolanaAddressGenerator;

impl AddressGenerator for SolanaAddressGenerator {
    fn generate(&self) -> Result<(String, Vec<u8>)> {
        let kp = Keypair::new();
        Ok((kp.pubkey().to_string(), kp.to_bytes().to_vec()))
    }
}

/// Solana transaction fee in lamports
pub const TX_FEE_LAMPORTS: u64 = 5_000;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solana_address_generator() {
        let gen = SolanaAddressGenerator;
        let (addr, bytes) = gen.generate().unwrap();
        assert!(addr.len() >= 43 && addr.len() <= 44); // Base58 Solana pubkey
        assert_eq!(bytes.len(), 64); // Solana keypair is 64 bytes
    }

    #[test]
    fn test_solana_address_unique() {
        let gen = SolanaAddressGenerator;
        let (a, _) = gen.generate().unwrap();
        let (b, _) = gen.generate().unwrap();
        assert_ne!(a, b);
    }
}
