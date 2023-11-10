use const_env::from_env;
use axum::{
  extract::Json,
  response::IntoResponse,
  routing::post,
  Router,
  http::StatusCode,
};
use std::net::SocketAddr;
use num_bigint::{BigUint};

use summa_backend::{MerkleSumTree, Entry, Node, Tree};

use serde::{Serialize, Deserialize};
 
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

#[from_env]
const N_ASSETS: usize = 2;
#[from_env]
const N_BYTES: usize = 14;

#[tokio::main]
async fn main() {
    // Define the app with a route
    let app = Router::new()
        .route("/", post(create_mst));

    // Define the address to serve on
    let addr = SocketAddr::from(([0, 0, 0, 0], 4000)); // TODO: assign ports from env variable 

    // Start the server
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

fn convert_node_to_json(node: &Node<N_ASSETS>) -> JsonNode {
  JsonNode {
      hash: format!("{:?}", node.hash),
      balances: node.balances.iter().map(|b| format!("{:?}", b)).collect(),
  }
}

async fn create_mst(Json(json_entries): Json<Vec<JsonEntry>>) -> Result<impl IntoResponse, (StatusCode, Json<JsonMerkleSumTree>)> {
    // Convert `JsonEntry` -> `Entry<N_ASSETS>`
    let entries = json_entries.iter().map(|entry| {
      let mut balances: [BigUint; N_ASSETS] = std::array::from_fn(|_| BigUint::from(0u32));
      entry.balances.iter().enumerate().for_each(|(i, balance)| {
        balances[i] = balance.parse::<BigUint>().unwrap();
      });
      Entry::new(entry.username.clone(), balances).unwrap()
    }).collect::<Vec<Entry<N_ASSETS>>>();

    // Create `MerkleSumTree<N_ASSETS, N_BYTES>` from `parsed_entries`
    let tree = MerkleSumTree::<N_ASSETS, N_BYTES>::from_entries(entries, false).unwrap();

    // Convert `MerkleSumTree<N_ASSETS, N_BYTES>` to `JsonMerkleSumTree`
    let json_tree = JsonMerkleSumTree {
      root: convert_node_to_json(&tree.root()),
      nodes: tree.nodes().iter().map(|layer| {
          layer.iter().map(convert_node_to_json).collect()
      }).collect(),
      depth: tree.depth().clone(),
      entries: tree.entries().iter().map(|entry| {
          JsonEntry {
              balances: entry.balances().iter().map(|b| b.to_string()).collect(),
              username: entry.username().to_string(),
          }
      }).collect(),
      is_sorted: false, // TODO: assign from request data
  };
    
    Ok((StatusCode::OK, Json(json_tree)))
}
