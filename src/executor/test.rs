#[cfg(test)]
mod test {
    use futures::future;
    use std::error::Error;

    use crate::executor::{spawner::ExecutorSpawner, MockSpawner};
    use crate::orchestrator::entry_parser;

    #[tokio::test]
    async fn test_executor() -> Result<(), Box<dyn Error>> {
        let spawner = MockSpawner::new(None);

        let executor = spawner.spawn_executor().await;

        let entries = entry_parser::<_, 2, 14>("./src/orchestrator/csv/entry_16.csv").unwrap();
        let merkle_tree = executor.generate_tree::<2, 14>(entries).await.unwrap();

        spawner.terminate_executors().await;

        assert_eq!(
            format!("{:?}", merkle_tree.root.hash),
            "0x02e021d9bf99c5bd7267488b6a7a5cf5f7d00222a41b6a9b971899c44089e0c5"
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_executor_block() -> Result<(), Box<dyn Error>> {
        let spawner = MockSpawner::new(None);
        let executor = spawner.spawn_executor().await;

        // Parse two csv files
        let entries_1 = entry_parser::<_, 2, 14>("./src/orchestrator/csv/entry_16.csv").unwrap();
        let entries_2 = entry_parser::<_, 2, 14>("./src/orchestrator/csv/entry_16.csv").unwrap();

        let merkle_tree_1 = executor.generate_tree::<2, 14>(entries_1);
        let merkle_tree_2 = executor.generate_tree::<2, 14>(entries_2);

        let all_tree = future::join_all([merkle_tree_1, merkle_tree_2]).await;

        spawner.terminate_executors().await;

        assert_eq!(all_tree.len(), 2);

        Ok(())
    }
}
