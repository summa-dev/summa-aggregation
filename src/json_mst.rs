use num_bigint::BigUint;
use serde::{Deserialize, Serialize};

use halo2_proofs::halo2curves::{bn256::Fr as Fp, group::ff::PrimeField};

use summa_backend::merkle_sum_tree::{Entry, MerkleSumTree, Node, Tree};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonEntry {
    pub username: String,
    pub balances: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonNode {
    pub hash: String,
    pub balances: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonMerkleSumTree {
    pub root: JsonNode,
    pub nodes: Vec<Vec<JsonNode>>,
    pub depth: usize,
    pub entries: Vec<JsonEntry>,
    pub is_sorted: bool,
}

pub fn convert_node_to_json<const N_ASSETS: usize>(node: &Node<N_ASSETS>) -> JsonNode {
    JsonNode {
        hash: format!("{:?}", node.hash),
        balances: node.balances.iter().map(|b| format!("{:?}", b)).collect(),
    }
}

fn parse_fp_from_hex(hex_str: &str) -> Fp {
    let bigint = BigUint::parse_bytes(hex_str[2..].as_bytes(), 16).unwrap();
    Fp::from_str_vartime(&bigint.to_str_radix(10)).unwrap()
}

impl JsonEntry {
    pub fn new(username: String, balances: Vec<String>) -> Self {
        JsonEntry { username, balances }
    }

    pub fn to_entry<const N_ASSETS: usize>(&self) -> Entry<N_ASSETS> {
        let mut balances: [BigUint; N_ASSETS] = std::array::from_fn(|_| BigUint::from(0u32));
        self.balances.iter().enumerate().for_each(|(i, balance)| {
            balances[i] = balance.parse::<BigUint>().unwrap();
        });

        Entry::<N_ASSETS>::new(self.username.clone(), balances).unwrap()
    }
}

impl JsonNode {
    pub fn to_node<const N_ASSETS: usize>(&self) -> Node<N_ASSETS> {
        let hash = parse_fp_from_hex(&self.hash);
        let balances = self
            .balances
            .iter()
            .map(|balance| parse_fp_from_hex(balance))
            .collect::<Vec<_>>()
            .try_into()
            .expect("Incorrect number of balances");

        Node { hash, balances }
    }
}

impl JsonMerkleSumTree {
    pub fn from_tree<const N_ASSETS: usize, const N_BYTES: usize>(
        tree: MerkleSumTree<N_ASSETS, N_BYTES>,
    ) -> Self {
        let root = convert_node_to_json(tree.root());
        let nodes = tree
            .nodes()
            .iter()
            .map(|node| node.iter().map(convert_node_to_json).collect())
            .collect();
        let entries = tree
            .entries()
            .iter()
            .map(|entry| {
                JsonEntry::new(
                    entry.username().to_string(),
                    entry.balances().iter().map(|b| b.to_string()).collect(),
                )
            })
            .collect();

        JsonMerkleSumTree {
            root,
            nodes,
            depth: *tree.depth(),
            entries,
            is_sorted: tree.is_sorted,
        }
    }

    pub fn to_mst<const N_ASSETS: usize, const N_BYTES: usize>(
        &self,
    ) -> MerkleSumTree<N_ASSETS, N_BYTES> {
        let root: Node<N_ASSETS> = self.root.to_node::<N_ASSETS>();
        let nodes = self
            .nodes
            .iter()
            .map(|node| node.iter().map(|n| n.to_node()).collect())
            .collect();
        let entries = self
            .entries
            .iter()
            .map(|entry| entry.to_entry::<N_ASSETS>())
            .collect();

        MerkleSumTree::<N_ASSETS, N_BYTES> {
            root,
            nodes,
            depth: self.depth,
            entries,
            is_sorted: self.is_sorted,
        }
    }
}
