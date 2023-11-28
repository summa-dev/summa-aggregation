use axum::{extract::Json, http::StatusCode, response::IntoResponse};
use const_env::from_env;
use num_bigint::BigUint;

use crate::{JsonEntry, JsonMerkleSumTree, JsonNode};
use summa_backend::merkle_sum_tree::{Entry, MerkleSumTree, Node, Tree};

#[from_env]
const N_ASSETS: usize = 2;
#[from_env]
const N_BYTES: usize = 14;

fn convert_node_to_json(node: &Node<N_ASSETS>) -> JsonNode {
    JsonNode {
        hash: format!("{:?}", node.hash),
        balances: node.balances.iter().map(|b| format!("{:?}", b)).collect(),
    }
}

pub async fn create_mst(
    Json(json_entries): Json<Vec<JsonEntry>>,
) -> Result<impl IntoResponse, (StatusCode, Json<JsonMerkleSumTree>)> {
    // Convert `JsonEntry` -> `Entry<N_ASSETS>`
    let entries = json_entries
        .iter()
        .map(|entry| {
            let mut balances: [BigUint; N_ASSETS] = std::array::from_fn(|_| BigUint::from(0u32));
            entry.balances.iter().enumerate().for_each(|(i, balance)| {
                balances[i] = balance.parse::<BigUint>().unwrap();
            });
            Entry::new(entry.username.clone(), balances).unwrap()
        })
        .collect::<Vec<Entry<N_ASSETS>>>();

    #[cfg(not(test))]
    let entries_length = entries.len();
    #[cfg(not(test))]
    let starting_time = std::time::Instant::now();

    // Create `MerkleSumTree<N_ASSETS, N_BYTES>` from `parsed_entries`
    let tree = MerkleSumTree::<N_ASSETS, N_BYTES>::from_entries(entries, false).unwrap();

    #[cfg(not(test))]
    println!(
        "Time to create tree({} entries): {}ms",
        entries_length,
        starting_time.elapsed().as_millis()
    );

    // Convert `MerkleSumTree<N_ASSETS, N_BYTES>` to `JsonMerkleSumTree`
    let json_tree = JsonMerkleSumTree {
        root: convert_node_to_json(&tree.root()),
        nodes: tree
            .nodes()
            .iter()
            .map(|layer| layer.iter().map(convert_node_to_json).collect())
            .collect(),
        depth: tree.depth().clone(),
        entries: tree
            .entries()
            .iter()
            .map(|entry| JsonEntry {
                balances: entry.balances().iter().map(|b| b.to_string()).collect(),
                username: entry.username().to_string(),
            })
            .collect(),
        is_sorted: false, // Always false because sorted entries inside minitree is meaningless
    };

    Ok((StatusCode::OK, Json(json_tree)))
}