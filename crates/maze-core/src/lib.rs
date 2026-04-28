//! Maze Core: chain-agnostic dynamic maze routing
//!
//! This crate provides the algorithm for generating maze topologies
//! used in privacy-preserving transaction routing. It is independent
//! of any specific blockchain.

pub mod address_gen;
pub mod error;
pub mod params;
pub mod topology;
pub mod utils;

pub use address_gen::AddressGenerator;
pub use error::{MazeError, Result};
pub use params::{MazeParameters, MergeStrategy, DelayPattern, DelayScope};
pub use topology::{MazeGenerator, MazeGraph, MazeNode};
