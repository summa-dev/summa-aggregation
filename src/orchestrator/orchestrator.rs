use std::cmp::min;
use tokio::sync::mpsc;

use super::entry_csv_parser::entry_parser;
use crate::aggregation_merkle_sum_tree::AggregationMerkleSumTree;
use crate::executor::ExecutorSpawner;

pub struct Orchestrator<const N_ASSETS: usize, const N_BYTES: usize> {
    // executors: Vec<Box<Executor>>, // TODO: Should it be thread safe
    executor_spawner: Box<dyn ExecutorSpawner>,
    entry_csvs: Vec<String>,
}

impl<const N_ASSETS: usize, const N_BYTES: usize> Orchestrator<N_ASSETS, N_BYTES> {
    fn new(spawner: Box<dyn ExecutorSpawner>, entry_csvs: Vec<String>) -> Self {
        Self {
            // executors: spawner.spawn_executor(number_executor),
            executor_spawner: spawner,
            entry_csvs,
        }
    }

    // Calculate the range of tasks to be assigned to a executor.
    //
    // * `executor_index` - The index of the executor.
    // * `total_executors` - The total number of executor.
    //
    // A tuple representing the start and end indices of the tasks assigned to the executor 
    fn calculate_task_range(
        &self,
        executor_index: usize,
        total_executors: usize,
    ) -> (usize, usize) {
        let total_tasks = self.entry_csvs.len();
        let base_tasks_per_executor = total_tasks / total_executors;
        let extra_tasks = total_tasks % total_executors;

        let start = executor_index * base_tasks_per_executor + min(executor_index, extra_tasks);
        let end =
            (executor_index + 1) * base_tasks_per_executor + min(executor_index + 1, extra_tasks);

        (start, min(end, total_tasks))
    }

    async fn create_aggregation_mst(
        self,
        number_executor: usize,
    ) -> Result<AggregationMerkleSumTree<N_ASSETS, N_BYTES>, Box<dyn std::error::Error>>
    where
        [usize; N_ASSETS + 1]: Sized,
        [usize; 2 * (1 + N_ASSETS)]: Sized,
    {
        let entries_per_executor = self.entry_csvs.len() / number_executor;

        // Declare channels for internal communication
        let mut executors = Vec::new();
        let mut result_collectors = Vec::new();

        let actual_number_of_workers = min(number_executor, self.entry_csvs.len());
        for i in 0..actual_number_of_workers {
            let (entries_tx, mut entries_rx) = mpsc::channel(32);
            let (tree_tx, tree_rx) = mpsc::channel(32);
            let executor = self.executor_spawner.spawn_executor();
            result_collectors.push((i, tree_rx));

            // Spawn executors that process entries
            executors.push(tokio::spawn(async move {
                while let Some(task) = entries_rx.recv().await {
                    let processed_task = executor
                        .generate_tree::<N_ASSETS, N_BYTES>(task)
                        .await
                        .unwrap();
                    if tree_tx.send(processed_task).await.is_err() {
                        break;
                    }
                }
            }));

            // Distribute path of entry csv files to executors
            let (start, end) = self.calculate_task_range(i, number_executor);
            let entry_csvs_slice = self.entry_csvs[start..end].to_vec(); // Clone only the necessary slice

            tokio::spawn(async move {
                for task in entry_csvs_slice.iter() {
                    let entries = entry_parser::<_, N_ASSETS, N_BYTES>(task).unwrap();
                    if entries_tx.send(entries).await.is_err() {
                        break;
                    }
                }
            });
        }

        // Collect results from executors
        let mut all_tree_responses = Vec::new();
        for (index, mut result_rx) in result_collectors {
            let executor_results = tokio::spawn(async move {
                let mut trees = Vec::new();
                while let Some(result) = result_rx.recv().await {
                    trees.push(result);
                }
                (index, trees)
            });
            all_tree_responses.push(executor_results);
        }

        // Wait for all workers to finish
        for executor in executors {
            let _ = executor.await;
        }

        // Aggregate results from all workers in order
        let mut aggregated_tree_results = vec![None; self.entry_csvs.len()];
        for result in all_tree_responses {
            let (index, worker_results) = result.await.unwrap();
            let start = index * entries_per_executor;
            for (i, res) in worker_results.iter().enumerate() {
                aggregated_tree_results[start + i] = Some(res.clone());
            }
        }

        // TODO: make sure the number of flattened data is correct
        let all_merkle_sum_tree = aggregated_tree_results.into_iter().flatten().collect();

        AggregationMerkleSumTree::new(all_merkle_sum_tree)
    }
}

#[cfg(test)]
mod test {
    use std::error::Error;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::Orchestrator;
    use crate::executor::{Executor, ExecutorSpawner};

    #[tokio::test]
    async fn test_predefiend_simple_spawner() -> Result<(), Box<dyn Error>> {
        // This is temporary until we have a better way to test this
        struct Spawner {
            port_counter: AtomicUsize
        }

        impl Spawner {
            pub fn new(start_port: usize) -> Self {
                Spawner {
                    port_counter: AtomicUsize::new(start_port),
                }
            }
        }

        impl ExecutorSpawner for Spawner {
            fn spawn_executor(&self) -> Executor {
                let port = self.port_counter.fetch_add(1, Ordering::SeqCst);
                Executor::new(format!("http://localhost:{}", port))
            }

            fn terminate_executor(&self, executor: Executor) {
                drop(executor);
            }
        }

        let spawner = Spawner::new(4000);

        let orchestrator = Orchestrator::<2, 14>::new(
            Box::new(spawner),
            vec![
                "./src/orchestrator/csv/entry_16.csv".to_string(),
                "./src/orchestrator/csv/entry_16.csv".to_string(),
            ],
        );
        let aggregation_merkle_sum_tree = orchestrator.create_aggregation_mst(2).await?;

        assert_eq!(16, aggregation_merkle_sum_tree.mini_tree(0).entries.len());
        assert_eq!(16, aggregation_merkle_sum_tree.mini_tree(1).entries.len());
        Ok(())
    }
}
