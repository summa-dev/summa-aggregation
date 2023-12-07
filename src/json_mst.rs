use num_bigint::BigUint;
use serde::{Deserialize, Serialize};
use std::error::Error;

use halo2_proofs::halo2curves::{bn256::Fr as Fp, group::ff::PrimeField};

use summa_backend::merkle_sum_tree::{Cryptocurrency, Entry, MerkleSumTree, Node, Tree};

/// JsonNode
/// Represents a entry in the Merkle Sum Tree in JSON format.
/// The balance in the Merkle Sum Tree was presented BigUint format, but in the JSON format, it is presented as a string.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonEntry {
    pub username: String,
    pub balances: Vec<String>,
}

/// JsonNode
/// Represents a node in the Merkle Sum Tree in JSON format.
/// The balance in the Merkle Sum Tree was presented BigUint format, but in the JSON format, it is presented as a string.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonNode {
    pub hash: String,
    pub balances: Vec<String>,
}

/// JsonMerkleSumTree
/// Represents the entire Merkle Sum Tree in JSON format.
/// It is used for transmitting tree data between the executor and mini-tree-server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonMerkleSumTree {
    pub root: JsonNode,
    pub nodes: Vec<Vec<JsonNode>>,
    pub depth: usize,
    pub entries: Vec<JsonEntry>,
    pub is_sorted: bool,
}

pub fn convert_node_to_json<const N_CURRENCIES: usize>(node: &Node<N_CURRENCIES>) -> JsonNode {
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

    /// Converts an `Entry` to a `JsonEntry`.
    ///
    /// This method translates an `Entry` into its JSON format.
    /// It is used by the Executor to send Entry data to the mini-tree-server in JSON format.
    pub fn from_entry<const N_CURRENCIES: usize>(entry: &Entry<N_CURRENCIES>) -> Self {
        JsonEntry::new(
            entry.username().to_string(),
            entry
                .balances()
                .iter()
                .map(|balance| balance.to_string())
                .collect(),
        )
    }

    /// Converts a `JsonEntry` back to an `Entry`.
    ///
    /// This method is utilized by the mini-tree-server when processing data received from the executor in JSON format.
    /// It converts `JsonEntry` objects back to the `Entry` struct, facilitating the construction of the Merkle Sum Tree.
    pub fn to_entry<const N_CURRENCIES: usize>(&self) -> Entry<N_CURRENCIES> {
        let mut balances: [BigUint; N_CURRENCIES] = std::array::from_fn(|_| BigUint::from(0u32));
        self.balances.iter().enumerate().for_each(|(i, balance)| {
            balances[i] = balance.parse::<BigUint>().unwrap();
        });

        Entry::<N_CURRENCIES>::new(self.username.clone(), balances).unwrap()
    }
}

/// Converts a `JsonNode` back to a `Node` for reconstructing the Merkle Sum Tree from JSON data.
impl JsonNode {
    pub fn to_node<const N_CURRENCIES: usize>(&self) -> Node<N_CURRENCIES> {
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
    /// Converts a MerkleSumTree to its JSON representation.
    ///
    /// This function is essential for the mini-tree-server to send the Merkle Sum Tree results back to the executor in JSON format,
    /// facilitating the translation of the tree structure into a universally readable JSON form.
    pub fn from_tree<const N_CURRENCIES: usize, const N_BYTES: usize>(
        tree: MerkleSumTree<N_CURRENCIES, N_BYTES>,
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
            is_sorted: false,
        }
    }

    /// Converts a JsonMerkleSumTree back to a MerkleSumTree.
    ///
    /// This function is crucial when handling data received in JSON format from the mini-tree-server.
    /// It rebuilds the MerkleSumTree on the main machine using the `from_params` method.
    /// This method is preferred over `from_entries` as the nodes are pre-computed by the mini-tree-server, thus the tree doesn't need to be recomputed from scratch.
    pub fn to_mst<const N_CURRENCIES: usize, const N_BYTES: usize>(
        &self,
    ) -> Result<MerkleSumTree<N_CURRENCIES, N_BYTES>, Box<dyn Error>>
    where
        [usize; N_CURRENCIES + 1]: Sized,
        [usize; N_CURRENCIES + 2]: Sized,
    {
        let root: Node<N_CURRENCIES> = self.root.to_node::<N_CURRENCIES>();
        let nodes = self
            .nodes
            .iter()
            .map(|node| node.iter().map(|n| n.to_node()).collect())
            .collect();
        let entries = self
            .entries
            .iter()
            .map(|entry| entry.to_entry::<N_CURRENCIES>())
            .collect();
        let cryptocurrencies = vec![
            Cryptocurrency {
                name: "Dummy".to_string(),
                chain: "ETH".to_string(),
            };
            N_CURRENCIES
        ];

        MerkleSumTree::<N_CURRENCIES, N_BYTES>::from_params(
            root,
            nodes,
            self.depth,
            entries,
            cryptocurrencies,
            self.is_sorted,
        )
    }
}
