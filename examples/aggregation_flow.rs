#![feature(generic_const_exprs)]
use axum::{routing::post, Router};
use std::error::Error;
use std::net::SocketAddr;

use summa_aggregation::{
    executor::CloudSpawner, mini_tree_generator::create_mst, orchestrator::Orchestrator,
};
use summa_backend::{
    apis::round::Round,
    contracts::signer::{AddressInput, SummaSigner},
    tests::initialize_test_env,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // In this example, we will use local mini-tree server to generate mini-tree
    let app = Router::new().route("/", post(create_mst));
    let addr = SocketAddr::from(([0, 0, 0, 0], 4000));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();

    // The CloudSpawner, when used with the Orchestrator, does not rely on a `docker-compose.yml` file or a `service_name` to create Workers.
    // It solely utilizes the `worker_node_url`. Typically, in production environments, workers operate on remote nodes.
    const LEVELS: usize = 2;
    const N_CURRENCIES: usize = 2;
    const N_BYTES: usize = 14;

    // For this demonstration, using the same URL address is acceptable. However, the number of URLs should match the number of executors.
    let worker_node_urls = vec!["127.0.0.1".to_string(), "127.0.0.1".to_string()];
    let spawner = CloudSpawner::new(None, worker_node_urls, 4000);
    let orchestrator = Orchestrator::<2, 14>::new(
        Box::new(spawner),
        vec![
            "./src/orchestrator/csv/entry_16.csv".to_string(),
            "./src/orchestrator/csv/entry_16.csv".to_string(),
        ],
    );
    // Number of Executor should be equal to number of worker_node_urls
    let aggregation_merkle_sum_tree = orchestrator.create_aggregation_mst(2).await.unwrap();

    // The remaining steps are similar to the `summa-backend` example, specifically 'summa_solvency_flow'.
    // For detailed information, refer to: https://github.com/summa-dev/summa-solvency/blob/master/backend/examples/summa_solvency_flow.rs
    let (anvil, _, _, _, summa_contract) = initialize_test_env(None).await;

    let signer = SummaSigner::new(
        "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80",
        anvil.endpoint().as_str(),
        AddressInput::Address(summa_contract.address()),
    )
    .await?;

    // Once get the aggregation merkle sum tree, we can initialize the round.
    let timestamp = 1u64;
    let params_path = "examples/hermez-raw-11";
    let round = Round::<LEVELS, N_CURRENCIES, N_BYTES>::new(
        &signer,
        Box::new(aggregation_merkle_sum_tree),
        params_path,
        timestamp,
    )
    .unwrap();

    // Next, generate the proof of inclusion.
    let inclusion_proof_of_user0 = round.get_proof_of_inclusion(0).unwrap();
    assert!(inclusion_proof_of_user0.get_public_inputs().len() > 0); // Check public input counts

    println!("Generated User 0 proof of inclusion");
    Ok(())
}
