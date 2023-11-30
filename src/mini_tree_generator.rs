use axum::{extract::Json, http::StatusCode, response::IntoResponse};
use const_env::from_env;

use crate::json_mst::{convert_node_to_json, JsonEntry, JsonMerkleSumTree};
use summa_backend::merkle_sum_tree::{Entry, MerkleSumTree, Tree};

#[from_env]
const N_ASSETS: usize = 2;
#[from_env]
const N_BYTES: usize = 14;

pub async fn create_mst(
    Json(json_entries): Json<Vec<JsonEntry>>,
) -> Result<impl IntoResponse, (StatusCode, Json<JsonMerkleSumTree>)> {
    // Convert `JsonEntry` -> `Entry<N_ASSETS>`
    let entries = json_entries
        .iter()
        .map(|entry| entry.to_entry())
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
    let json_tree = JsonMerkleSumTree::from_tree(tree);

    Ok((StatusCode::OK, Json(json_tree)))
}
