use axum::{routing::post, Router};
use std::net::SocketAddr;

use summa_aggregation::mini_tree_generator::create_mst;

#[tokio::main]
async fn main() {
    // Define the app with a route
    let app = Router::new().route("/", post(create_mst));

    // Define the address to serve on
    let addr = SocketAddr::from(([0, 0, 0, 0], 4000));

    // Start the server
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
