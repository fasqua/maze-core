# KausaLayer Maze Core

Chain-agnostic dynamic maze routing library for privacy-preserving transaction routing.

## Overview

Maze Core implements the dynamic maze routing algorithm from KausaLayer as a standalone,
reusable library. The algorithm generates unpredictable transaction topologies using
ephemeral wallets, golden ratio splits, fibonacci merge patterns, and amount noise
to make transaction flow analysis practically impossible.

This is the core privacy primitive powering KausaLayer on Solana.

## Architecture

The project is organized as a Cargo workspace with three crates:

| Crate | Description |
|-------|-------------|
| `maze-core` | Chain-agnostic algorithm. No blockchain dependencies. |
| `maze-solana` | Solana adapter. Implements `AddressGenerator` using Solana keypairs. |
| `maze-example` | End-to-end demo showing maze generation with Solana keypairs. |

## How It Works

Dynamic maze routing creates a multi-level graph of ephemeral wallets:

1. A deposit node receives the funds
2. Funds are split across multiple paths using golden ratio variations
3. Paths merge and split again across configurable hop levels (5-10)
4. Amount noise is applied at each node to obscure patterns
5. All paths converge to a final node for delivery

Every wallet in the maze is ephemeral (generated fresh, used once). Every routing
topology is unique. No two routes share the same pattern.

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
maze-core = "0.1"
maze-solana = "0.1"  # For Solana integration
```

Generate a maze route:

```rust
use maze_core::{MazeGenerator, MazeParameters};
use maze_solana::SolanaAddressGenerator;

let params = MazeParameters::default();
let generator = MazeGenerator::new(params);
let addr_gen = SolanaAddressGenerator;

let encrypt_fn = |data: &[u8]| -> maze_core::error::Result<Vec<u8>> {
    // Your encryption logic here
    Ok(data.to_vec())
};

let maze = generator.generate(1_000_000_000, encrypt_fn, &addr_gen).unwrap();
println!("Nodes: {}, Transactions: {}",
    maze.nodes.len(), maze.total_transactions);
```

## Custom Chain Integration

To use maze routing on a different chain, implement the `AddressGenerator` trait:

```rust
use maze_core::AddressGenerator;
use maze_core::error::Result;

struct MyChainAddressGenerator;

impl AddressGenerator for MyChainAddressGenerator {
    fn generate(&self) -> Result<(String, Vec<u8>)> {
        // Generate a keypair for your chain
        // Return (address_string, private_key_bytes)
        todo!()
    }
}
```

That is the only integration point. One trait, one method.

## Parameters

| Parameter | Default | Range | Description |
|-----------|---------|-------|-------------|
| `hop_count` | 7 | 5-10 | Number of routing levels |
| `split_ratio` | 1.618 | 1.1-3.0 | Golden ratio variant for fund splitting |
| `merge_strategy` | Random | Early/Late/Middle/Random/Fibonacci | When to merge paths |
| `delay_pattern` | Random | None/Linear/Exponential/Random/Fibonacci | Timing between transactions |
| `amount_noise` | 0.5% | 0-1% | Amount obfuscation percentage |
| `tx_fee` | 5000 | >0 | Transaction fee in base units |

## Build & Test

```bash
cargo build --workspace
cargo test --workspace
cargo run -p maze-example
```

## License

Apache-2.0

## Links

- Website: https://kausalayer.com
- Twitter: https://x.com/kausalayer
- GitHub: https://github.com/fasqua/maze-core
