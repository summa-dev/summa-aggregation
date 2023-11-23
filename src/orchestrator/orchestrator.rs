use std::cmp::min;
use tokio::sync::mpsc;

use super::entry_csv_parser::entry_parser;
use crate::aggregation_merkle_sum_tree::AggregationMerkleSumTree;
use crate::executor::ExecutorSpawner;

pub struct Orchestrator<const N_ASSETS: usize, const N_BYTES: usize> {
    executor_spawner: Box<dyn ExecutorSpawner>,
    entry_csvs: Vec<String>,
}

impl<const N_ASSETS: usize, const N_BYTES: usize> Orchestrator<N_ASSETS, N_BYTES> {
    fn new(executor_spawner: Box<dyn ExecutorSpawner>, entry_csvs: Vec<String>) -> Self {
        Self {
            executor_spawner,
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

    /// Processes a list of CSV files concurrently using executors and aggregates the results.
    ///
    /// * `number_executor` - The number of executors to use.
    ///
    /// Data flow
    ///
    /// 1. Splits the list of CSV files into segments based on the number of available executors.
    /// 2. A distribution thread loads each CSV file, parses it into `entries`, and sends these to `entries_tx`.
    /// 3. Each executor receives `entries` from `entries_rx`, requests tasks to Worker, and sends results back through `tree_tx`.
    /// 4. The processed data from all executors, collected from `tree_rx`, is aggregated into an `AggregationMerkleSumTree`.
    /// 6. After processing, executors are terminated to release resources.
    ///
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
        // One of the channels receives parsed data from entry parser to distribute tasks to executor
        // while the other channel is used the executors that send a result of the tasks.
        let mut executors = Vec::new();
        let mut result_collectors = Vec::new();

        let actual_number_of_workers = min(number_executor, self.entry_csvs.len());
        for i in 0..actual_number_of_workers {
            let (entries_tx, mut entries_rx) = mpsc::channel(32);
            let (tree_tx, tree_rx) = mpsc::channel(32);
            let executor = self.executor_spawner.spawn_executor().await;
            result_collectors.push((i, tree_rx));

            // 2. Executor
            //
            // Spawn executors that process entries with Worker.
            //
            // - Receives 'entries' from [entries_rx] channel.
            // - Processes 'entries' to build a tree (done by worker).
            // - Sends the resulting 'tree' back via [tree_tx] channel.
            //
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

            // 1. Distributing Tasks
            //
            // Spawn a distribution thread that distributes entries to executors
            //
            // - Loads CSV file from [csv_file_path].
            // - Parses CSV file into 'entries'.
            // - Sends 'entries' to executors via [entries_tx] channel.
            //
            let (start, end) = self.calculate_task_range(i, number_executor);
            let entry_csvs_slice = self.entry_csvs[start..end].to_vec(); // Clone only the necessary slice

            tokio::spawn(async move {
                for file_path in entry_csvs_slice.iter() {
                    let entries = entry_parser::<_, N_ASSETS, N_BYTES>(file_path).unwrap();
                    if entries_tx.send(entries).await.is_err() {
                        break;
                    }
                }
            });
        }

        // 3. Collectoing Results
        //
        // Collect `tree` results from executors
        //
        //  - Receives processed 'tree' from [tree_rx] channel.
        //  - Collects all 'tree' results into 'worker_results'.
        //  - Aggregates 'worker_results' into 'aggregated_tree_results'.
        //
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

        // Terminate executors
        self.executor_spawner.terminate_executors().await;

        let all_merkle_sum_tree = aggregated_tree_results.into_iter().flatten().collect();

        AggregationMerkleSumTree::new(all_merkle_sum_tree)
    }
}
