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
    // 1. Setup Distributed Environment
    //
    // Custodians can use any cloud infrastructure to set up worker nodes.
    // In this example, instead of spawning worker containers on remote nodes, we will use two local servers running mini-tree services as workers.

    // Spawning Worker_1
    tokio::spawn(async move {
        let app = Router::new().route("/", post(create_mst));
        let addr = SocketAddr::from(([0, 0, 0, 0], 4000));
        axum::Server::bind(&addr)
            .serve(app.into_make_service())
            .await
            .unwrap();
    });

    // Spawning Worker_2
    tokio::spawn(async move {
        let app = Router::new().route("/", post(create_mst));
        let addr = SocketAddr::from(([0, 0, 0, 0], 4001));
        axum::Server::bind(&addr)
            .serve(app.into_make_service())
            .await
            .unwrap();
    });

    // We assume that custodians, when setting up their distributed environment, will obtain the URLs of worker nodes.
    // In this example, we use two worker URLs corresponding to the workers spawned earlier.
    // It is important to ensure that the number of URLs matches the number of executors.
    let worker_node_urls = vec!["127.0.0.1:4000".to_string(), "127.0.0.1:4001".to_string()];

    // To initiate the Round, a SummaSigner instance and its corresponding SummaContract instance are required.
    // Here, we initialize the signer with a specified private key and the Summa contract's address.
    let (anvil, _, _, _, summa_contract) = initialize_test_env(None).await;
    let signer = SummaSigner::new(
        "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80",
        anvil.endpoint().as_str(),
        AddressInput::Address(summa_contract.address()),
    )
    .await?;
    // Up to this point, the above code logic can be viewed as the Custodian's cloud infra structure setup process for Summa-Aggregation.

    // 2. Initialize the Round with Aggregtaion Merkle Sum Tree
    //
    // Setting parameters for the aggregation merkle sum tree:
    //
    // LEVELS: This defines the number of levels in the aggregation merkle sum tree, including the mini-tree level.
    const LEVELS: usize = 5;
    // N_CURRENCIES: Specifies the number of currencies in the entry data.
    const N_CURRENCIES: usize = 2;
    // N_BYTES: Determines the maximum total balance allowed for each currency, calculated as 1 << (8 * 14) = 2^112.
    const N_BYTES: usize = 14;
    // Note: These parameters should match those in the Summa contract.

    // CloudSpawner does not depend on a `docker-compose.yml` file or a `service_name` for creating workers.
    // This implies that `service_info` is not necessary. When `service_info` is absent, CloudSpawner creates an Executor solely based on the `worker_node_url`.
    let spawner = CloudSpawner::new(None, worker_node_urls, 4000);
    let orchestrator = Orchestrator::<N_CURRENCIES, N_BYTES>::new(
        Box::new(spawner),
        vec![
            "csv/entry_16_1.csv".to_string(),
            "csv/entry_16_2.csv".to_string(),
        ],
    );

    // The number of Executors must match the number of worker_node_urls.
    let aggregation_merkle_sum_tree = orchestrator.create_aggregation_mst(2).await.unwrap();

    // After obtaining the aggregation merkle sum tree, we proceed to initialize the round.
    let timestamp = 1u64;
    let params_path = "examples/hermez-raw-11";
    let round = Round::<LEVELS, N_CURRENCIES, N_BYTES>::new(
        &signer,
        Box::new(aggregation_merkle_sum_tree),
        params_path,
        timestamp,
    )
    .unwrap();

    // 3. Interact with the Summa Contract and Generate Proof of Inclusion
    //
    // Interactions with the Summa contract, such as sending Commitment or AddressOwnership, are similar to those in the `summa-backend` example, particularly 'summa_solvency_flow'.
    // For detailed information, refer to the example at: https://github.com/summa-dev/summa-solvency/blob/master/backend/examples/summa_solvency_flow.rs
    //
    // Here, we demonstrate generating the proof of inclusion for User 0.
    let inclusion_proof_of_user0 = round.get_proof_of_inclusion(0).unwrap();
    assert!(!inclusion_proof_of_user0.get_public_inputs().is_empty()); // Check public input counts

    println!("Generated User 0 proof of inclusion");
    Ok(())
}
