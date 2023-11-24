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
    ) -> Result<MerkleSumTree<N_ASSETS, N_BYTES>, Box<dyn Error>> {
        // Parse the response body into a MerkleSumTree
        let json_tree = self
            .client
            .post(&self.url)
            .json(&json_entries)
            .send()
            .await?
            .json::<JsonMerkleSumTree>()
            .await?;

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

#[cfg(test)]
mod test {
    use futures::future;
    use std::{error::Error, sync::atomic::AtomicUsize};

    use bollard::Docker;
    use super::Executor;
    use crate::orchestrator::entry_parser;
    use crate::executor::ContainerSpawner;
    use crate::executor::spawner::ExecutorSpawner;

    #[tokio::test]
    async fn test_executor() -> Result<(), Box<dyn Error>> {
        let spawner = ContainerSpawner::new(
            "summa-aggregation".to_string(),
            "mini_tree_generator".to_string(),
        );

        let executor = spawner.spawn_executor().await;

        let entries = entry_parser::<_, 2, 14>("./src/orchestrator/csv/entry_16.csv").unwrap();
        let merkle_tree = executor.generate_tree::<2, 14>(entries).await?;

        assert_eq!(
            format!("{:?}", merkle_tree.root.hash),
            "0x02e021d9bf99c5bd7267488b6a7a5cf5f7d00222a41b6a9b971899c44089e0c5"
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_executor_block() -> Result<(), Box<dyn Error>> {
        let spawner = ContainerSpawner::new(
            "summa-aggregation".to_string(),
            "mini_tree_generator".to_string(),
        );

        let executor = spawner.spawn_executor().await;

        // Parse two csv files
        let entries_1 = entry_parser::<_, 2, 14>("./src/orchestrator/csv/entry_16.csv").unwrap();
        let entries_2 = entry_parser::<_, 2, 14>("./src/orchestrator/csv/entry_16.csv").unwrap();

        let merkle_tree_1 = executor.generate_tree::<2, 14>(entries_1);
        let merkle_tree_2 = executor.generate_tree::<2, 14>(entries_2);

        let all_tree = future::join_all([merkle_tree_1, merkle_tree_2]).await;

        assert_eq!(all_tree.len(), 2);

        Ok(())
    }
}
