#![allow(unused_imports)]
use crate::executor::{CloudSpawner, LocalSpawner, MockSpawner};
use crate::orchestrator::Orchestrator;
use summa_backend::merkle_sum_tree::Tree;

#[tokio::test]
async fn test_single_mock_worker() {
    let spawner = MockSpawner::new(None);

    let orchestrator = Orchestrator::<2, 14>::new(
        Box::new(spawner),
        vec![
            "./src/orchestrator/csv/entry_16_1.csv".to_string(),
            "./src/orchestrator/csv/entry_16_2.csv".to_string(),
        ],
    );
    let aggregation_merkle_sum_tree = orchestrator.create_aggregation_mst(1).await.unwrap();

    assert_eq!(16, aggregation_merkle_sum_tree.mini_tree(0).entries().len());
    assert_eq!(16, aggregation_merkle_sum_tree.mini_tree(1).entries().len());
}

#[tokio::test]
async fn test_none_exist_csv() {
    let spawner = MockSpawner::new(None);
    let orchestrator = Orchestrator::<2, 14>::new(
        Box::new(spawner),
        vec![
            "./src/orchestrator/csv/entry_16.csv".to_string(),
            "./src/orchestrator/csv/no_exist.csv".to_string(),
        ],
    );
    match orchestrator.create_aggregation_mst(2).await {
        Ok(_) => panic!("Expected an error"),
        Err(e) => {
            assert!(e
                .to_string()
                .contains("Mismatch in generated mini tree counts and given CSV counts"));
        }
    }
}

#[tokio::test]
async fn test_none_exist_worker() {
    let non_exist_worker_url = vec!["127.0.0.1:40".to_string()]; // unsignable port
    let spawner = MockSpawner::new(Some(non_exist_worker_url));

    let orchestrator = Orchestrator::<2, 14>::new(
        Box::new(spawner),
        vec![
            "./src/orchestrator/csv/entry_16_1.csv".to_string(),
            "./src/orchestrator/csv/entry_16_2.csv".to_string(),
        ],
    );

    match orchestrator.create_aggregation_mst(2).await {
        Ok(_) => panic!("Expected an error"),
        Err(e) => {
            assert!(e
                .to_string()
                .contains("Mismatch in generated mini tree counts and given CSV counts"));
        }
    }
}

#[cfg(feature = "docker")]
#[tokio::test]
async fn test_with_containers() {
    let spawner = LocalSpawner::new(
        "summadev/summa-aggregation-mini-tree:latest".to_string(),
        "orchestrator_test".to_string(),
    );

    let orchestrator = Orchestrator::<2, 14>::new(
        Box::new(spawner),
        vec![
            "./src/orchestrator/csv/entry_16_1.csv".to_string(),
            "./src/orchestrator/csv/entry_16_2.csv".to_string(),
        ],
    );
    let aggregation_merkle_sum_tree = orchestrator.create_aggregation_mst(2).await.unwrap();

    assert_eq!(16, aggregation_merkle_sum_tree.mini_tree(0).entries().len());
    assert_eq!(16, aggregation_merkle_sum_tree.mini_tree(1).entries().len());
}

#[cfg(feature = "docker-swarm")]
#[tokio::test]
async fn test_with_swarm_service() {
    let spawner = CloudSpawner::new(
        Some(("mini_tree".to_string(), "docker-compose.yml".to_string())),
        vec!["10.0.0.1".to_string(), "10.0.0.2".to_string()],
        4000,
    );

    let orchestrator = Orchestrator::<2, 14>::new(
        Box::new(spawner),
        vec![
            "./src/orchestrator/csv/entry_16_1.csv".to_string(),
            "./src/orchestrator/csv/entry_16_2.csv".to_string(),
        ],
    );
    let aggregation_merkle_sum_tree = orchestrator.create_aggregation_mst(2).await.unwrap();
    assert_eq!(16, aggregation_merkle_sum_tree.mini_tree(0).entries().len());
    assert_eq!(16, aggregation_merkle_sum_tree.mini_tree(1).entries().len());
}
