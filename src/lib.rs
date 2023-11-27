#![feature(generic_const_exprs)]
pub mod aggregation_merkle_sum_tree;
pub mod executor;
pub mod mini_tree_generator;
pub mod orchestrator;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonNode {
    pub hash: String,
    pub balances: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonEntry {
    pub balances: Vec<String>,
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonMerkleSumTree {
    pub root: JsonNode,
    pub nodes: Vec<Vec<JsonNode>>,
    pub depth: usize,
    pub entries: Vec<JsonEntry>,
    pub is_sorted: bool,
}
