//! Maze topology generator
//!
//! Generates a dynamic maze graph for privacy-preserving transaction routing.
//! The topology consists of multiple splits and merges across levels,
//! parameterized with an encrypted seed for deterministic but unpredictable
//! routing patterns.
//!
//! This module is fully chain-agnostic. Chain-specific address generation
//! is injected via the AddressGenerator trait.

use serde::{Deserialize, Serialize};

use crate::address_gen::AddressGenerator;
use crate::error::{MazeError, Result};
use crate::params::{MazeParameters, MergeStrategy};
use crate::utils::{add_noise, fibonacci, seeded_random};

/// A node in the maze graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MazeNode {
    /// Unique index of this node
    pub index: u16,
    /// Level/depth in the maze (0 = deposit, max = final)
    pub level: u8,
    /// Address string in the chain's native format
    pub address: String,
    /// Encrypted signing material bytes
    pub signing_material_encrypted: Vec<u8>,
    /// Incoming edges (node indices that send to this node)
    pub inputs: Vec<u16>,
    /// Outgoing edges (node indices this node sends to)
    pub outputs: Vec<u16>,
    /// Amount to receive (in base units, e.g. lamports)
    pub amount_in: u64,
    /// Amount to send out (after TX fee)
    pub amount_out: u64,
    /// Transaction signature for incoming TX
    pub tx_in_signature: Option<String>,
    /// Transaction signatures for outgoing TXs
    pub tx_out_signatures: Vec<Option<String>>,
    /// Status: pending, completed, failed
    pub status: String,
}

/// The complete maze graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MazeGraph {
    /// All nodes in the maze
    pub nodes: Vec<MazeNode>,
    /// Maze parameters used for generation
    pub parameters: MazeParameters,
    /// Total levels in maze
    pub total_levels: u8,
    /// Deposit node index (always 0)
    pub deposit_index: u16,
    /// Final node index (sends to destination)
    pub final_index: u16,
    /// Total TX count
    pub total_transactions: u16,
}

/// Maze Generator
///
/// Generates maze topologies using the provided parameters and
/// address generator. The same parameters + seed will always
/// produce the same topology structure (deterministic).
pub struct MazeGenerator {
    params: MazeParameters,
}

impl MazeGenerator {
    pub fn new(params: MazeParameters) -> Self {
        Self { params }
    }

    pub fn with_random_params() -> Self {
        Self {
            params: MazeParameters::random(),
        }
    }

    /// Generate a maze graph for a transfer.
    ///
    /// # Arguments
    /// * `total_amount` - Total amount to transfer (in base units)
    /// * `encrypt_fn` - Function to encrypt signing material bytes
    /// * `addr_gen` - Chain-specific address generator
    ///
    /// # Returns
    /// * `MazeGraph` - The generated maze structure
    pub fn generate<F>(
        &self,
        total_amount: u64,
        encrypt_fn: F,
        addr_gen: &dyn AddressGenerator,
    ) -> Result<MazeGraph>
    where
        F: Fn(&[u8]) -> Result<Vec<u8>>,
    {
        let mut nodes: Vec<MazeNode> = Vec::new();
        let mut node_index: u16 = 0;

        // Calculate total TX fees needed
        let estimated_txs = self.estimate_transaction_count();
        let total_fees = self.params.tx_fee * estimated_txs as u64;

        if total_amount <= total_fees {
            return Err(MazeError::InsufficientFunds {
                required: total_fees + 1,
                available: total_amount,
            });
        }

        let net_amount = total_amount - total_fees;

        // Level 0: Deposit node
        let (deposit_addr, deposit_bytes) = addr_gen.generate()
            .map_err(|e| MazeError::AddressGeneration(e.to_string()))?;
        let deposit_node = MazeNode {
            index: node_index,
            level: 0,
            address: deposit_addr,
            signing_material_encrypted: encrypt_fn(&deposit_bytes)?,
            inputs: vec![],
            outputs: vec![],
            amount_in: total_amount,
            amount_out: 0,
            tx_in_signature: None,
            tx_out_signatures: vec![],
            status: "pending".to_string(),
        };
        nodes.push(deposit_node);
        node_index += 1;

        let num_levels = self.params.hop_count;
        let has_pool = self.params.pool_address.is_some()
            && self.params.pool_signing_material.is_some();

        if has_pool {
            // === POOL MODE: Pre-pool hops -> Pool -> Post-pool hops ===
            let pool_address = self.params.pool_address.as_ref().unwrap().clone();
            let pool_sm_bytes = self.params.pool_signing_material.as_ref().unwrap().clone();

            // Split hop_count: pre-pool gets ~half, post-pool gets the rest
            let pre_pool_levels = (num_levels / 2).max(2);
            let post_pool_levels = num_levels.saturating_sub(pre_pool_levels).max(2);

            // --- Pre-pool hops ---
            let mut current_level_nodes: Vec<u16> = vec![0];
            let mut current_level_amounts: Vec<u64> = vec![net_amount];

            for level in 1..=pre_pool_levels {
                let (new_nodes, new_amounts) = self.generate_level(
                    level,
                    &current_level_nodes,
                    &current_level_amounts,
                    &mut node_index,
                    &encrypt_fn,
                    &mut nodes,
                    addr_gen,
                )?;
                current_level_nodes = new_nodes;
                current_level_amounts = new_amounts;
            }

            // --- Pool node: merge all pre-pool paths into pool ---
            let pool_level = pre_pool_levels + 1;
            let pool_amount: u64 = current_level_amounts.iter().sum();
            let pool_node = MazeNode {
                index: node_index,
                level: pool_level,
                address: pool_address,
                signing_material_encrypted: encrypt_fn(&pool_sm_bytes)?,
                inputs: current_level_nodes.clone(),
                outputs: vec![],
                amount_in: pool_amount,
                amount_out: 0,
                tx_in_signature: None,
                tx_out_signatures: vec![],
                status: "pending".to_string(),
            };

            let pool_index = node_index;
            nodes.push(pool_node);
            node_index += 1;

            // Update pre-pool nodes outputs to point to pool
            for &prev_idx in &current_level_nodes {
                if let Some(node) = nodes.get_mut(prev_idx as usize) {
                    node.outputs.push(pool_index);
                }
            }

            // --- Post-pool hops: pool is the new starting point ---
            let mut current_level_nodes: Vec<u16> = vec![pool_index];
            let mut current_level_amounts: Vec<u64> = vec![pool_amount];

            for i in 1..=post_pool_levels {
                let level = pool_level + i;
                let (new_nodes, new_amounts) = self.generate_level(
                    level,
                    &current_level_nodes,
                    &current_level_amounts,
                    &mut node_index,
                    &encrypt_fn,
                    &mut nodes,
                    addr_gen,
                )?;
                current_level_nodes = new_nodes;
                current_level_amounts = new_amounts;
            }

            // --- Final node ---
            let final_level = pool_level + post_pool_levels + 1;
            let (final_addr, final_bytes) = addr_gen.generate()
                .map_err(|e| MazeError::AddressGeneration(e.to_string()))?;
            let final_amount: u64 = current_level_amounts.iter().sum();
            let final_node = MazeNode {
                index: node_index,
                level: final_level,
                address: final_addr,
                signing_material_encrypted: encrypt_fn(&final_bytes)?,
                inputs: current_level_nodes.clone(),
                outputs: vec![],
                amount_in: final_amount,
                amount_out: final_amount.saturating_sub(self.params.tx_fee),
                tx_in_signature: None,
                tx_out_signatures: vec![],
                status: "pending".to_string(),
            };

            let final_index = node_index;
            nodes.push(final_node);

            for &prev_idx in &current_level_nodes {
                if let Some(node) = nodes.get_mut(prev_idx as usize) {
                    node.outputs.push(final_index);
                }
            }

            self.calculate_amounts(&mut nodes)?;
            let total_transactions = self.count_transactions(&nodes);

            Ok(MazeGraph {
                nodes,
                parameters: self.params.clone(),
                total_levels: final_level + 1,
                deposit_index: 0,
                final_index,
                total_transactions,
            })
        } else {
            // === STANDARD MODE: No pool, original behavior ===
            let mut current_level_nodes: Vec<u16> = vec![0];
            let mut current_level_amounts: Vec<u64> = vec![net_amount];

            for level in 1..num_levels {
                let (new_nodes, new_amounts) = self.generate_level(
                    level,
                    &current_level_nodes,
                    &current_level_amounts,
                    &mut node_index,
                    &encrypt_fn,
                    &mut nodes,
                    addr_gen,
                )?;
                current_level_nodes = new_nodes;
                current_level_amounts = new_amounts;
            }

            // Final level: Merge all to single node
            let (final_addr, final_bytes) = addr_gen.generate()
                .map_err(|e| MazeError::AddressGeneration(e.to_string()))?;
            let final_amount: u64 = current_level_amounts.iter().sum();
            let final_node = MazeNode {
                index: node_index,
                level: num_levels,
                address: final_addr,
                signing_material_encrypted: encrypt_fn(&final_bytes)?,
                inputs: current_level_nodes.clone(),
                outputs: vec![],
                amount_in: final_amount,
                amount_out: final_amount.saturating_sub(self.params.tx_fee),
                tx_in_signature: None,
                tx_out_signatures: vec![],
                status: "pending".to_string(),
            };

            let final_index = node_index;
            nodes.push(final_node);

            // Update outputs for previous level nodes
            for &prev_idx in &current_level_nodes {
                if let Some(node) = nodes.get_mut(prev_idx as usize) {
                    node.outputs.push(final_index);
                }
            }

            // Calculate amount_out for all nodes
            self.calculate_amounts(&mut nodes)?;

            // Count total transactions
            let total_transactions = self.count_transactions(&nodes);

            Ok(MazeGraph {
                nodes,
                parameters: self.params.clone(),
                total_levels: num_levels + 1,
                deposit_index: 0,
                final_index,
                total_transactions,
            })
        }
    }

    /// Generate nodes for a single level
    fn generate_level<F>(
        &self,
        level: u8,
        prev_nodes: &[u16],
        prev_amounts: &[u64],
        node_index: &mut u16,
        encrypt_fn: &F,
        nodes: &mut Vec<MazeNode>,
        addr_gen: &dyn AddressGenerator,
    ) -> Result<(Vec<u16>, Vec<u64>)>
    where
        F: Fn(&[u8]) -> Result<Vec<u8>>,
    {
        let mut new_node_indices: Vec<u16> = Vec::new();
        let mut new_amounts: Vec<u64> = Vec::new();

        // Determine split/merge behavior based on strategy and level
        let should_split = self.should_split_at_level(level);
        let should_merge = self.should_merge_at_level(level);

        if should_split && prev_nodes.len() < 4 {
            // Split: Each node splits into 2-3 nodes
            for (i, (&prev_idx, &amount)) in prev_nodes.iter().zip(prev_amounts.iter()).enumerate() {
                let split_count = self.get_split_count(level, i as u64);
                let split_amounts = self.split_amount(amount, split_count);

                for (j, split_amount) in split_amounts.into_iter().enumerate() {
                    let (addr, sm_bytes) = addr_gen.generate()
                        .map_err(|e| MazeError::AddressGeneration(e.to_string()))?;
                    let noised_amount = add_noise(
                        split_amount,
                        self.params.amount_noise,
                        &self.params.seed,
                        (*node_index as u64) * 1000 + j as u64,
                    );

                    let node = MazeNode {
                        index: *node_index,
                        level,
                        address: addr,
                        signing_material_encrypted: encrypt_fn(&sm_bytes)?,
                        inputs: vec![prev_idx],
                        outputs: vec![],
                        amount_in: noised_amount,
                        amount_out: 0,
                        tx_in_signature: None,
                        tx_out_signatures: vec![],
                        status: "pending".to_string(),
                    };

                    // Update previous node's outputs
                    if let Some(prev_node) = nodes.get_mut(prev_idx as usize) {
                        prev_node.outputs.push(*node_index);
                    }

                    new_node_indices.push(*node_index);
                    new_amounts.push(noised_amount);
                    nodes.push(node);
                    *node_index += 1;
                }
            }
        } else if should_merge && prev_nodes.len() > 2 {
            // Merge: Combine multiple nodes into fewer
            let merge_groups = self.create_merge_groups(prev_nodes, prev_amounts);

            for (inputs, amounts) in merge_groups {
                let (addr, sm_bytes) = addr_gen.generate()
                    .map_err(|e| MazeError::AddressGeneration(e.to_string()))?;
                let total: u64 = amounts.iter().sum();

                let node = MazeNode {
                    index: *node_index,
                    level,
                    address: addr,
                    signing_material_encrypted: encrypt_fn(&sm_bytes)?,
                    inputs: inputs.clone(),
                    outputs: vec![],
                    amount_in: total,
                    amount_out: 0,
                    tx_in_signature: None,
                    tx_out_signatures: vec![],
                    status: "pending".to_string(),
                };

                // Update previous nodes' outputs
                for &prev_idx in &inputs {
                    if let Some(prev_node) = nodes.get_mut(prev_idx as usize) {
                        prev_node.outputs.push(*node_index);
                    }
                }

                new_node_indices.push(*node_index);
                new_amounts.push(total);
                nodes.push(node);
                *node_index += 1;
            }
        } else {
            // Pass through: 1-to-1 mapping
            for (&prev_idx, &amount) in prev_nodes.iter().zip(prev_amounts.iter()) {
                let (addr, sm_bytes) = addr_gen.generate()
                    .map_err(|e| MazeError::AddressGeneration(e.to_string()))?;
                let noised_amount = add_noise(
                    amount,
                    self.params.amount_noise,
                    &self.params.seed,
                    *node_index as u64,
                );

                let node = MazeNode {
                    index: *node_index,
                    level,
                    address: addr,
                    signing_material_encrypted: encrypt_fn(&sm_bytes)?,
                    inputs: vec![prev_idx],
                    outputs: vec![],
                    amount_in: noised_amount,
                    amount_out: 0,
                    tx_in_signature: None,
                    tx_out_signatures: vec![],
                    status: "pending".to_string(),
                };

                if let Some(prev_node) = nodes.get_mut(prev_idx as usize) {
                    prev_node.outputs.push(*node_index);
                }

                new_node_indices.push(*node_index);
                new_amounts.push(noised_amount);
                nodes.push(node);
                *node_index += 1;
            }
        }

        Ok((new_node_indices, new_amounts))
    }

    /// Determine if we should split at this level
    fn should_split_at_level(&self, level: u8) -> bool {
        let ratio = level as f64 / self.params.hop_count as f64;

        match self.params.merge_strategy {
            MergeStrategy::Early => ratio > 0.5,
            MergeStrategy::Late => ratio < 0.5,
            MergeStrategy::Middle => ratio < 0.3 || ratio > 0.7,
            MergeStrategy::Fibonacci => {
                let fib_idx = (level + self.params.fib_offset) % 20;
                fibonacci(fib_idx) % 2 == 0
            }
            MergeStrategy::Random => {
                seeded_random(&self.params.seed, level as u64) % 2 == 0
            }
        }
    }

    /// Determine if we should merge at this level
    fn should_merge_at_level(&self, level: u8) -> bool {
        let ratio = level as f64 / self.params.hop_count as f64;

        match self.params.merge_strategy {
            MergeStrategy::Early => ratio < 0.5,
            MergeStrategy::Late => ratio > 0.5,
            MergeStrategy::Middle => ratio > 0.3 && ratio < 0.7,
            MergeStrategy::Fibonacci => {
                let fib_idx = (level + self.params.fib_offset) % 20;
                fibonacci(fib_idx) % 2 == 1
            }
            MergeStrategy::Random => {
                seeded_random(&self.params.seed, level as u64 + 1000) % 2 == 0
            }
        }
    }

    /// Get number of splits for a node
    fn get_split_count(&self, level: u8, node_idx: u64) -> usize {
        let rand_val = seeded_random(&self.params.seed, level as u64 * 100 + node_idx);
        let base = 2 + (rand_val % 2) as usize;
        base.min(4)
    }

    /// Split amount into parts using golden ratio
    fn split_amount(&self, amount: u64, parts: usize) -> Vec<u64> {
        if parts <= 1 {
            return vec![amount];
        }

        let mut amounts = Vec::with_capacity(parts);
        let mut remaining = amount;

        for i in 0..(parts - 1) {
            let ratio = self.params.split_ratio + (i as f64 * 0.1);
            let part = (remaining as f64 / ratio) as u64;
            let part = part.max(self.params.tx_fee * 2);
            amounts.push(part);
            remaining = remaining.saturating_sub(part);
        }

        amounts.push(remaining);
        amounts
    }

    /// Create merge groups from previous nodes
    fn create_merge_groups(&self, prev_nodes: &[u16], prev_amounts: &[u64]) -> Vec<(Vec<u16>, Vec<u64>)> {
        let mut groups: Vec<(Vec<u16>, Vec<u64>)> = Vec::new();

        let mut i = 0;
        while i < prev_nodes.len() {
            if i + 1 < prev_nodes.len() {
                groups.push((
                    vec![prev_nodes[i], prev_nodes[i + 1]],
                    vec![prev_amounts[i], prev_amounts[i + 1]],
                ));
                i += 2;
            } else {
                groups.push((
                    vec![prev_nodes[i]],
                    vec![prev_amounts[i]],
                ));
                i += 1;
            }
        }

        groups
    }

    /// Calculate amount_out for all nodes
    fn calculate_amounts(&self, nodes: &mut Vec<MazeNode>) -> Result<()> {
        for i in 0..nodes.len() {
            let output_count = nodes[i].outputs.len();
            if output_count > 0 {
                let total_fees = self.params.tx_fee * output_count as u64;
                let amount_in = nodes[i].amount_in;
                nodes[i].amount_out = amount_in.saturating_sub(total_fees);
            }
        }
        Ok(())
    }

    /// Count total transactions in the maze
    fn count_transactions(&self, nodes: &[MazeNode]) -> u16 {
        nodes.iter()
            .map(|n| n.outputs.len() as u16)
            .sum::<u16>()
            + 1
    }

    /// Estimate transaction count before generation
    fn estimate_transaction_count(&self) -> u16 {
        let avg_branch = (self.params.split_ratio * 1.5) as u16;
        (self.params.hop_count as u16 * avg_branch).max(10)
    }
}

impl MazeGraph {
    /// Get execution order (topological sort by level)
    pub fn get_execution_order(&self) -> Vec<&MazeNode> {
        let mut ordered: Vec<&MazeNode> = self.nodes.iter().collect();
        ordered.sort_by_key(|n| n.level);
        ordered
    }

    /// Get nodes at a specific level
    pub fn get_nodes_at_level(&self, level: u8) -> Vec<&MazeNode> {
        self.nodes.iter().filter(|n| n.level == level).collect()
    }

    /// Get deposit node
    pub fn get_deposit_node(&self) -> Option<&MazeNode> {
        self.nodes.get(self.deposit_index as usize)
    }

    /// Get final node
    pub fn get_final_node(&self) -> Option<&MazeNode> {
        self.nodes.get(self.final_index as usize)
    }

    /// Check if all nodes are completed
    pub fn is_completed(&self) -> bool {
        self.nodes.iter().all(|n| n.status == "completed")
    }

    /// Get progress (completed nodes / total nodes)
    pub fn get_progress(&self) -> (usize, usize) {
        let completed = self.nodes.iter().filter(|n| n.status == "completed").count();
        (completed, self.nodes.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::address_gen::DummyAddressGenerator;

    fn dummy_encrypt(data: &[u8]) -> Result<Vec<u8>> {
        Ok(data.to_vec())
    }

    #[test]
    fn test_maze_generation() {
        let params = MazeParameters::default();
        let generator = MazeGenerator::new(params);
        let addr_gen = DummyAddressGenerator;

        let amount = 1_000_000_000;
        let maze = generator.generate(amount, dummy_encrypt, &addr_gen).unwrap();

        assert!(maze.nodes.len() >= 2);
        assert_eq!(maze.deposit_index, 0);
        assert!(maze.final_index > 0);
        assert!(maze.total_transactions > 0);
    }

    #[test]
    fn test_maze_topology() {
        let mut params = MazeParameters::default();
        params.hop_count = 5;
        let generator = MazeGenerator::new(params);
        let addr_gen = DummyAddressGenerator;

        let amount = 5_000_000_000;
        let maze = generator.generate(amount, dummy_encrypt, &addr_gen).unwrap();

        // Check deposit node
        let deposit = maze.get_deposit_node().unwrap();
        assert_eq!(deposit.level, 0);
        assert!(deposit.inputs.is_empty());

        // Check final node
        let final_node = maze.get_final_node().unwrap();
        assert!(final_node.outputs.is_empty());
    }

    #[test]
    fn test_execution_order() {
        let params = MazeParameters::default();
        let generator = MazeGenerator::new(params);
        let addr_gen = DummyAddressGenerator;

        let maze = generator.generate(1_000_000_000, dummy_encrypt, &addr_gen).unwrap();
        let order = maze.get_execution_order();

        let mut prev_level = 0;
        for node in order {
            assert!(node.level >= prev_level);
            prev_level = node.level;
        }
    }

    #[test]
    fn test_insufficient_funds() {
        let params = MazeParameters::default();
        let generator = MazeGenerator::new(params);
        let addr_gen = DummyAddressGenerator;

        let result = generator.generate(100, dummy_encrypt, &addr_gen);
        assert!(result.is_err());
    }
}
