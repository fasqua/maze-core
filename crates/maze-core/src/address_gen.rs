//! Address generator trait
//!
//! This trait is the single abstraction point between the chain-agnostic
//! maze algorithm and chain-specific implementations. Each chain only
//! needs to implement one method: generate().

use crate::error::Result;

/// Trait for chain-specific address generation.
///
/// Implementors generate a new address with corresponding signing material
/// (e.g. private key bytes) for use as intermediate maze nodes.
///
/// # Example
///
/// ```rust,ignore
/// use maze_core::AddressGenerator;
/// use maze_core::error::Result;
///
/// struct MyChainAddressGenerator;
///
/// impl AddressGenerator for MyChainAddressGenerator {
///     fn generate(&self) -> Result<(String, Vec<u8>)> {
///         // Generate a new keypair for your chain
///         let address = "some_address".to_string();
///         let signing_bytes = vec![0u8; 32];
///         Ok((address, signing_bytes))
///     }
/// }
/// ```
pub trait AddressGenerator: Send + Sync {
    /// Generate a new address with corresponding signing material.
    ///
    /// # Returns
    /// A tuple of:
    /// - `String`: the address in the chain's native format
    /// - `Vec<u8>`: signing material (e.g. private key bytes) that will
    ///   be encrypted by the consumer for later transaction signing
    fn generate(&self) -> Result<(String, Vec<u8>)>;
}

/// Dummy generator for testing.
///
/// Generates fake hex addresses with random 32-byte signing material.
/// Not for production use.
#[cfg(any(test, feature = "testing"))]
pub struct DummyAddressGenerator;

#[cfg(any(test, feature = "testing"))]
impl AddressGenerator for DummyAddressGenerator {
    fn generate(&self) -> Result<(String, Vec<u8>)> {
        let bytes: [u8; 32] = rand::random();
        let address = bytes.iter().map(|b| format!("{:02x}", b)).collect::<String>();
        Ok((address, bytes.to_vec()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dummy_generator() {
        let gen = DummyAddressGenerator;
        let (addr, bytes) = gen.generate().unwrap();
        assert_eq!(addr.len(), 64, "hex address should be 64 chars");
        assert_eq!(bytes.len(), 32, "signing material should be 32 bytes");
    }

    #[test]
    fn test_dummy_generator_unique() {
        let gen = DummyAddressGenerator;
        let (addr_a, _) = gen.generate().unwrap();
        let (addr_b, _) = gen.generate().unwrap();
        assert_ne!(addr_a, addr_b, "each call should produce unique address");
    }
}
