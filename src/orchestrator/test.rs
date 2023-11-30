#[cfg(test)]
mod test {
    use crate::executor::{LocalSpawner, MockSpawner};
    use crate::orchestrator::Orchestrator;
    use summa_backend::merkle_sum_tree::Tree;

    #[tokio::test]
    async fn test_with_containers() {
        let spawner = LocalSpawner::new(
            "summa-aggregation".to_string(),
            "orchestrator_test".to_string(),
        );

        let orchestrator = Orchestrator::<2, 14>::new(
            Box::new(spawner),
            vec![
                "./src/orchestrator/csv/entry_16.csv".to_string(),
                "./src/orchestrator/csv/entry_16.csv".to_string(),
            ],
        );
        let aggregation_merkle_sum_tree = orchestrator.create_aggregation_mst(2).await.unwrap();
        assert_eq!(16, aggregation_merkle_sum_tree.mini_tree(0).entries.len());
        assert_eq!(16, aggregation_merkle_sum_tree.mini_tree(1).entries.len());
    }

    #[tokio::test]
    async fn test_single_mock_worker() {
        let spawner = MockSpawner::new(None);

        let orchestrator = Orchestrator::<2, 14>::new(
            Box::new(spawner),
            vec![
                "./src/orchestrator/csv/entry_16.csv".to_string(),
                "./src/orchestrator/csv/entry_16.csv".to_string(),
            ],
        );
        let aggregation_merkle_sum_tree = orchestrator.create_aggregation_mst(1).await.unwrap();
        assert_eq!(16, aggregation_merkle_sum_tree.mini_tree(0).entries.len());
        assert_eq!(16, aggregation_merkle_sum_tree.mini_tree(1).entries.len());
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
        let one_mini_tree_result = orchestrator.create_aggregation_mst(2).await.unwrap();
        assert_eq!(&0, one_mini_tree_result.depth());
    }

    #[tokio::test]
    async fn test_none_exist_worker() {
        let non_exist_worker_url = vec!["127.0.0.1:7878".to_string()];
        let spawner = MockSpawner::new(Some(non_exist_worker_url));

        let orchestrator = Orchestrator::<2, 14>::new(
            Box::new(spawner),
            vec![
                "./src/orchestrator/csv/entry_16.csv".to_string(),
                "./src/orchestrator/csv/entry_16.csv".to_string(),
            ],
        );
        let empty_mini_tree_error = orchestrator.create_aggregation_mst(2).await.unwrap_err();
        assert_eq!("Empty mini tree inputs", empty_mini_tree_error.to_string());
    }
}
