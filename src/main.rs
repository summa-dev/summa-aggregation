use summa_aggregation::{
    aggregation_merkle_sum_tree::AggregationMerkleSumTree, entry_parser, executor::Executor,
};
use summa_backend::merkle_sum_tree::{MerkleSumTree, Tree};
use tokio;
use futures::future;

async fn test(tasks: Vec<(String, String)>) {

    // This script is not based on docker swarm mode
    const N_ASSETS: usize = 2;
    const N_BYTES: usize = 14;

    let starting_time = std::time::Instant::now();
    let tree_futures = tasks
        .iter()
        .map(|(file, url)| {
            let csv_file = file.clone();
            let url = url.clone();
            
            tokio::spawn(async move {
                let entries = entry_parser::<_, 2, 14>(&csv_file).unwrap();
                let executor = Executor::new(url.to_string());
                
                // Assuming generate_tree now returns some data of interest
                let result = executor.generate_tree::<N_ASSETS, N_BYTES>(entries).await.unwrap();
                result
            })
        });

    let resp_futures = future::join_all(tree_futures).await;
    
    // Process the results
    let mut trees: Vec<MerkleSumTree<N_ASSETS, N_BYTES>> = Vec::new();
    for result in resp_futures {
        match result {
            Ok(data) => {
                // Handle successful data
                trees.push(data);
            }
            Err(e) => {
                // Handle errors
                println!("Error: {:?}", e);
            }
        }
    }
    let aggregation_tree = AggregationMerkleSumTree::new(trees).unwrap();

    let elapsed = starting_time.elapsed();
    println!("Tasks: {}, Root hash: {:?} with in {:?}", tasks.len(), aggregation_tree.root(), elapsed);
}

#[tokio::main]
async fn main() {
    // Big Task
    let big_single_task = vec![
        ("./src/data/entry_2_13.csv".to_string(),
        "http://localhost:4000".to_string())  
    ];

    test(big_single_task).await;

    // Chunked small tasks
    // the worker urls are defined in docker-compose.yml
    let small_chunked_tasks = vec![
        (
            "./src/data/entry_2_11_1.csv".to_string(),
            "http://localhost:4001".to_string(),
        ),
        (
            "./src/data/entry_2_11_2.csv".to_string(),
            "http://localhost:4004".to_string(),
        ),
        (
            "./src/data/entry_2_11_3.csv".to_string(),
            "http://localhost:4002".to_string(),
        ),
        (
            "./src/data/entry_2_11_4.csv".to_string(),
            "http://localhost:4003".to_string(),
        ),
    ];

    test(small_chunked_tasks).await;
}
