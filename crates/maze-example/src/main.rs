//! Example: Generate a maze route using maze-core + maze-solana
//!
//! This demonstrates end-to-end usage of the library.

use maze_core::{MazeGenerator, MazeParameters};
use maze_solana::SolanaAddressGenerator;

fn main() {
    println!("=== Maze Core Example ===");

    // Use default parameters (7 hops, golden ratio split, random merge)
    let params = MazeParameters::default();
    println!("Parameters: {} hops, split_ratio={:.3}, tx_fee={}",
        params.hop_count, params.split_ratio, params.tx_fee);

    let generator = MazeGenerator::new(params);
    let addr_gen = SolanaAddressGenerator;

    // Simple encrypt: identity function (for demo only)
    let encrypt_fn = |data: &[u8]| -> maze_core::error::Result<Vec<u8>> {
        Ok(data.to_vec())
    };

    let amount = 1_000_000_000; // 1 SOL in lamports
    match generator.generate(amount, encrypt_fn, &addr_gen) {
        Ok(maze) => {
            println!("Maze generated successfully:");
            println!("  Nodes: {}", maze.nodes.len());
            println!("  Levels: {}", maze.total_levels);
            println!("  Transactions: {}", maze.total_transactions);
            println!("  Deposit address: {}", maze.get_deposit_node().unwrap().address);
            println!("  Final address: {}", maze.get_final_node().unwrap().address);

            // Print each level
            for level in 0..maze.total_levels {
                let nodes = maze.get_nodes_at_level(level);
                println!("  Level {}: {} node(s)", level, nodes.len());
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }
}
