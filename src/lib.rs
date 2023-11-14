#![feature(generic_const_exprs)]
pub mod aggregation_merkle_sum_tree;
pub mod executor;
pub mod orchestrator;

pub use aggregation_merkle_sum_tree::AggregationMerkleSumTree;
pub use executor::Executor;
pub use orchestrator::entry_parser;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonNode {
    pub hash: String,
    pub balances: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonEntry {
    balances: Vec<String>,
    username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonMerkleSumTree {
    root: JsonNode,
    nodes: Vec<Vec<JsonNode>>,
    depth: usize,
    entries: Vec<JsonEntry>,
    is_sorted: bool,
}
