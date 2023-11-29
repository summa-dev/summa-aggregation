mod cloud_spawner;
mod local_spawner;
mod mock_spawner;
mod spawner;
mod test;

pub use cloud_spawner::CouldSpawner;
pub use local_spawner::LocalSpawner;
pub use mock_spawner::MockSpawner;
pub use spawner::ExecutorSpawner;

use halo2_proofs::halo2curves::{bn256::Fr as Fp, group::ff::PrimeField};
use num_bigint::BigUint;
use reqwest::Client;
use std::error::Error;

use crate::{JsonEntry, JsonMerkleSumTree, JsonNode};
use summa_backend::merkle_sum_tree::{Entry, MerkleSumTree, Node};

#[derive(Clone)]
pub struct Executor {
    client: Client,
    url: String,
    id: Option<String>,
}

impl Executor {
    pub fn new(url: String, id: Option<String>) -> Self {
        Executor {
            client: Client::new(),
            url,
            id,
        }
    }

    pub fn get_url(&self) -> String {
        self.url.clone()
    }

    pub fn get_name(&self) -> Option<String> {
        self.id.clone()
    }

    fn parse_fp_from_hex(hex_str: &str) -> Fp {
        let bigint = BigUint::parse_bytes(&hex_str[2..].as_bytes(), 16).unwrap();
        Fp::from_str_vartime(&bigint.to_str_radix(10)).unwrap()
    }

    fn convert_json_to_node<const N_ASSETS: usize>(json_node: JsonNode) -> Node<N_ASSETS> {
        let hash = Self::parse_fp_from_hex(&json_node.hash);
        let balances = json_node
            .balances
            .iter()
            .map(|balance| Self::parse_fp_from_hex(balance))
            .collect::<Vec<_>>()
            .try_into()
            .expect("Incorrect number of balances");

        Node { hash, balances }
    }

    pub async fn generate_tree<const N_ASSETS: usize, const N_BYTES: usize>(
        &self,
        json_entries: Vec<JsonEntry>,
    ) -> Result<MerkleSumTree<N_ASSETS, N_BYTES>, Box<dyn Error + Send>> {
        let response = self
            .client
            .post(&self.url)
            .json(&json_entries)
            .send()
            .await
            .map_err(|err| Box::new(err) as Box<dyn Error + Send>)?;

        let json_tree = response
            .json::<JsonMerkleSumTree>()
            .await
            .map_err(|err| Box::new(err) as Box<dyn Error + Send>)?;

        let entries = json_entries
            .iter()
            .map(|entry| {
                let mut balances: [BigUint; N_ASSETS] =
                    std::array::from_fn(|_| BigUint::from(0u32));
                entry.balances.iter().enumerate().for_each(|(i, balance)| {
                    balances[i] = balance.parse::<BigUint>().unwrap();
                });

                Entry::<N_ASSETS>::new(entry.username.clone(), balances).unwrap()
            })
            .collect::<Vec<Entry<N_ASSETS>>>();

        // Convert JsonMerkleSumTree to MerkleSumTree<N_ASSETS, N_BYTES>
        let tree = MerkleSumTree::<N_ASSETS, N_BYTES> {
            root: Self::convert_json_to_node(json_tree.root),
            nodes: json_tree
                .nodes
                .iter()
                .map(|nodes| {
                    nodes
                        .iter()
                        .map(|node| Self::convert_json_to_node(node.clone()))
                        .collect()
                })
                .collect(),
            depth: json_tree.depth,
            entries,
            is_sorted: false,
        };

        Ok(tree)
    }
}
