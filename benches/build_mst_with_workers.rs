#![feature(generic_const_exprs)]
use const_env::from_env;
use std::{error::Error, fs};
use summa_aggregation::{executor::CloudSpawner, orchestrator::Orchestrator};
use tokio::time::Instant;

#[from_env]
const LEVELS: usize = 20;
#[from_env]
const CHUNK: usize = 32;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // We assume that custodians, when setting up their distributed environment, will obtain the URLs of worker nodes.
    // In this example, we use two worker URLs corresponding to the workers spawned earlier.
    // It is important to ensure that the number of URLs matches the number of executors.
    let worker_node_urls = vec!["127.0.0.1:4000".to_string()];
    let total_workers = worker_node_urls.len();

    const N_CURRENCIES: usize = 1;
    const N_BYTES: usize = 14;
        
    // Read the directory and collect CSV file paths
    let csv_directory = format!("benches/csv/level_{}/{}_chunks", LEVELS, CHUNK);
    let csv_file_paths: Vec<String> = fs::read_dir(csv_directory)?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.is_file() && path.extension()? == "csv" {
                Some(path.to_string_lossy().into_owned())
            } else {
                None
            }
        })
        .collect();

    println!(
        "LEVELS: {}, N_CURRENCIES: {}, number_of_csv: {}, number_of_workers: {}, ",
        LEVELS,
        N_CURRENCIES,
        csv_file_paths.len(),
        worker_node_urls.len()
    );

    // The number of Executors must match the number of worker_node_urls.
    let start = Instant::now();

    let spawner = CloudSpawner::new(None, worker_node_urls.clone(), 4000);
    
    let orchestrator =
        Orchestrator::<N_CURRENCIES, N_BYTES>::new(Box::new(spawner), csv_file_paths);

    let _aggregation_merkle_sum_tree = orchestrator
        .create_aggregation_mst(worker_node_urls.len())
        .await
        .unwrap();
    println!(
        "Time to create aggregation merkle sum tree: {:?} s",
        start.elapsed()
    );
    Ok(())
}
