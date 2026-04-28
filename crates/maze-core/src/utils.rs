//! Utility functions for Maze Core
//!
//! Pure deterministic functions used by the maze topology generator.
//! No chain-specific dependencies.

use sha2::{Sha256, Digest};

/// Generate a deterministic random number from seed and index.
///
/// Uses SHA-256 to derive a pseudorandom u64 from the given seed
/// and index. Same seed + index always produces the same output.
pub fn seeded_random(seed: &[u8], index: u64) -> u64 {
    let mut hasher = Sha256::new();
    hasher.update(seed);
    hasher.update(&index.to_le_bytes());
    let result = hasher.finalize();
    u64::from_le_bytes(result[0..8].try_into().unwrap())
}

/// Get fibonacci number at index.
///
/// Returns the n-th Fibonacci number. Values for n <= 10 are
/// lookup-cached for performance. Used by MergeStrategy::Fibonacci
/// to determine split/merge patterns.
pub fn fibonacci(n: u8) -> u64 {
    match n {
        0 => 0,
        1 => 1,
        2 => 1,
        3 => 2,
        4 => 3,
        5 => 5,
        6 => 8,
        7 => 13,
        8 => 21,
        9 => 34,
        10 => 55,
        _ => {
            let mut a = 0u64;
            let mut b = 1u64;
            for _ in 0..n {
                let temp = a + b;
                a = b;
                b = temp;
            }
            b
        }
    }
}

/// Add noise to an amount based on seed.
///
/// Applies a deterministic pseudo-random noise within +/- noise_percent
/// of the original amount. Used to obfuscate transaction amounts so
/// that split patterns are not easily recognizable.
pub fn add_noise(amount: u64, noise_percent: f64, seed: &[u8], index: u64) -> u64 {
    let random = seeded_random(seed, index);
    let noise_range = (amount as f64 * noise_percent / 100.0) as u64;
    if noise_range == 0 {
        return amount;
    }
    let noise = (random % (noise_range * 2)) as i64 - noise_range as i64;
    (amount as i64 + noise).max(0) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seeded_random_deterministic() {
        let seed = [42u8; 32];
        let a = seeded_random(&seed, 0);
        let b = seeded_random(&seed, 0);
        assert_eq!(a, b, "same seed + index must produce same result");
    }

    #[test]
    fn test_seeded_random_different_index() {
        let seed = [42u8; 32];
        let a = seeded_random(&seed, 0);
        let b = seeded_random(&seed, 1);
        assert_ne!(a, b, "different index should produce different result");
    }

    #[test]
    fn test_fibonacci_known_values() {
        assert_eq!(fibonacci(0), 0);
        assert_eq!(fibonacci(1), 1);
        assert_eq!(fibonacci(5), 5);
        assert_eq!(fibonacci(10), 55);
        assert_eq!(fibonacci(15), 987);
        assert_eq!(fibonacci(20), 10946);
    }

    #[test]
    fn test_add_noise_deterministic() {
        let seed = [7u8; 32];
        let amount = 1_000_000;
        let a = add_noise(amount, 0.5, &seed, 0);
        let b = add_noise(amount, 0.5, &seed, 0);
        assert_eq!(a, b, "same inputs must produce same noise");
    }

    #[test]
    fn test_add_noise_within_range() {
        let seed = [99u8; 32];
        let amount = 1_000_000u64;
        let noise_percent = 1.0;
        for i in 0..100 {
            let noised = add_noise(amount, noise_percent, &seed, i);
            let max_noise = (amount as f64 * noise_percent / 100.0) as u64;
            let diff = (noised as i64 - amount as i64).unsigned_abs();
            assert!(diff <= max_noise, "noise {} exceeds max {}", diff, max_noise);
        }
    }

    #[test]
    fn test_add_noise_zero_percent() {
        let seed = [0u8; 32];
        let amount = 500_000;
        let noised = add_noise(amount, 0.0, &seed, 0);
        assert_eq!(noised, amount, "zero noise should return original amount");
    }
}
