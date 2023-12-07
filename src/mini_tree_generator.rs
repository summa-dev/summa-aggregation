use axum::{extract::Json, http::StatusCode, response::IntoResponse};
use const_env::from_env;

use crate::json_mst::{JsonEntry, JsonMerkleSumTree};
use summa_backend::merkle_sum_tree::{Cryptocurrency, Entry, MerkleSumTree};

/// Mini Tree Generator is designed to create Merkle Sum Tree using the Axum web framework. 
/// It primarily handles HTTP requests to generate tree based on provided JSON entries.
///
/// Constants:
/// - `N_CURRENCIES`: The number of cryptocurrencies involved. Set via environment variables.
/// - `N_BYTES`: The byte size for each entry. Set via environment variables.
///
/// Functions:
/// - `create_mst`: An asynchronous function that processes incoming JSON requests to generate a Merkle Sum Tree.
///   It converts `JsonEntry` objects into `Entry<N_CURRENCIES>` instances and then constructs the `MerkleSumTree`.
///   The function handles the conversion of the `MerkleSumTree` into a JSON format (`JsonMerkleSumTree`) for the response.
///
#[from_env]
const N_CURRENCIES: usize = 2;
#[from_env]
const N_BYTES: usize = 14;

pub async fn create_mst(
    Json(json_entries): Json<Vec<JsonEntry>>,
) -> Result<impl IntoResponse, (StatusCode, Json<JsonMerkleSumTree>)> {
    // Convert `JsonEntry` -> `Entry<N_CURRENCIES>`
    let entries = json_entries
        .iter()
        .map(|json_entry| json_entry.to_entry())
        .collect::<Vec<Entry<N_CURRENCIES>>>();
    let crypcocurrencies = vec![
        Cryptocurrency {
            name: "DUMMY".to_string(),
            chain: "ETH".to_string(),
        };
        N_CURRENCIES
    ];

    #[cfg(not(test))]
    let entries_length = entries.len();
    #[cfg(not(test))]
    let starting_time = std::time::Instant::now();

    // Create `MerkleSumTree<N_CURRENCIES, N_BYTES>` from `parsed_entries`
    let tree =
        MerkleSumTree::<N_CURRENCIES, N_BYTES>::from_entries(entries, crypcocurrencies, false)
            .unwrap();

    #[cfg(not(test))]
    println!(
        "Time to create tree({} entries): {}ms",
        entries_length,
        starting_time.elapsed().as_millis()
    );

    // Convert `MerkleSumTree<N_CURRENCIES, N_BYTES>` to `JsonMerkleSumTree`
    let json_tree = JsonMerkleSumTree::from_tree(tree);

    Ok((StatusCode::OK, Json(json_tree)))
}
