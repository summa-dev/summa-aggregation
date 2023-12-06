mod test;

use futures::future::join_all;
use std::{cmp::min, error::Error};
use summa_backend::merkle_sum_tree::{utils::parse_csv_to_entries, Cryptocurrency, MerkleSumTree};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::aggregation_merkle_sum_tree::AggregationMerkleSumTree;
use crate::executor::ExecutorSpawner;
use crate::json_mst::JsonEntry;

pub struct Orchestrator<const N_CURRENCIES: usize, const N_BYTES: usize> {
    executor_spawner: Box<dyn ExecutorSpawner>,
    entry_csvs: Vec<String>,
}

impl<const N_CURRENCIES: usize, const N_BYTES: usize> Orchestrator<N_CURRENCIES, N_BYTES> {
    pub fn new(executor_spawner: Box<dyn ExecutorSpawner>, entry_csvs: Vec<String>) -> Self {
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
    /// * `executor_count` - The number of executors to use.
    ///
    /// Data flow
    ///
    /// 1. Splits the list of CSV files into segments based on the number of available executors.
    /// 2. A distribution thread loads each CSV file, parses it into `entries`, and sends these to `entries_tx`.
    /// 3. Each executor receives `entries` from `entries_rx`, requests tasks to Worker, and sends results back through `tree_tx`.
    /// 4. The processed data from all executors, collected from `tree_rx`, is aggregated into an `AggregationMerkleSumTree`.
    /// 5. After processing, executors are terminated to release resources.
    ///
    pub async fn create_aggregation_mst(
        self,
        executor_count: usize,
    ) -> Result<AggregationMerkleSumTree<N_CURRENCIES, N_BYTES>, Box<dyn Error>>
    where
        [usize; N_CURRENCIES + 1]: Sized,
        [usize; N_CURRENCIES + 2]: Sized,
    {
        let entries_per_executor = self.entry_csvs.len() / executor_count;

        let mut executors = Vec::new();
        let mut result_collectors = Vec::new();

        let channel_size = std::env::var("CHANNEL_SIZE")
            .unwrap_or_default()
            .parse::<usize>()
            .unwrap_or(32);

        let cancel_token = CancellationToken::new();
        let actual_number_of_workers = min(executor_count, self.entry_csvs.len());
        for i in 0..actual_number_of_workers {
            // Declare channels for communication
            //
            // There are three channels are used inthis method.
            //
            // - A `entries_tx` receives parsed data from the entry parser to distribute tasks to executors.
            // - A `tree_tx` channel is used by the executors to send the results of the tasks.
            //
            let (entries_tx, mut entries_rx) = mpsc::channel(channel_size);
            let (tree_tx, tree_rx) = mpsc::channel(channel_size);
            // Executor
            //
            // Spawn executors that process entries with Worker.
            //
            // - Receives 'entries' from [entries_rx] channel.
            // - Processes 'entries' to build a merkle sum tree (done by worker).
            // - Sends the resulting 'tree' back via [tree_tx] channel.
            //
            let executor = self.executor_spawner.spawn_executor().await;
            result_collectors.push((i, tree_rx));

            let cloned_cancel_token = cancel_token.clone();
            executors.push(tokio::spawn(async move {
                        loop {
                            tokio::select! {
                                entries_data = entries_rx.recv() => {
                                    // When the distribution thread is finished, the channel will be closed.
                                    let entries = match entries_data {
                                        Some(entries) => entries,
                                        None => break,
                                    };
                                    let processed_task = match executor.generate_tree::<N_CURRENCIES, N_BYTES>(entries).await {
                                        Ok(entries) => entries,
                                        Err(e) => {
                                            eprintln!("Executor_{:?}: error while processing entries {:?}", i, e);
                                            cloned_cancel_token.cancel();
                                            break;
                                        }
                                    };
                                    if tree_tx.send(processed_task).await.is_err() {
                                        eprintln!("Executor_{:?}: Error while sending tree result", i);
                                        cloned_cancel_token.cancel();
                                        break;
                                    }
                                },
                                _ = cloned_cancel_token.cancelled() => {
                                    eprintln!("Executor_{:?}: cancel signal received, terminating.", i);
                                    break;
                                },
                            }
                        }
            }));

            // Distributing Tasks
            //
            // Spawn a distribution thread that distributes entries to executors
            //
            // - Loads CSV file from [csv_file_path].
            // - Parses CSV file into 'entries'.
            // - Sends 'entries' to executors via [entries_tx] channel.
            //
            let (start, end) = self.calculate_task_range(i, executor_count);
            let entry_csvs_slice = self.entry_csvs[start..end].to_vec(); // Clone only the necessary slice

            let cloned_cancel_token = cancel_token.clone();
            tokio::spawn(async move {
                for file_path in entry_csvs_slice.iter() {
                    let entries = match parse_csv_to_entries::<_, N_CURRENCIES, N_BYTES>(file_path)
                    {
                        Ok((_, entries)) => entries
                            .iter()
                            .map(JsonEntry::from_entry)
                            .collect::<Vec<JsonEntry>>(),
                        Err(e) => {
                            eprintln!(
                                "Executor_{:?}: Error while processing file {:?}: {:?}",
                                i, file_path, e
                            );
                            cloned_cancel_token.cancel();
                            break;
                        }
                    };

                    tokio::select! {
                        _ = cloned_cancel_token.cancelled() => {
                            eprintln!("Executor_{:?}: cancel signal received, terminating distributor.", i);
                            break;
                        },
                        send_entries = entries_tx.send(entries) => {
                            if let Err(e) = send_entries {
                                eprintln!("Executor_{:?}: Error while sending entries: {:?}", i, e);
                                cloned_cancel_token.cancel();
                                break;
                            }
                        }
                    }
                }
                drop(entries_tx);
            });
        }

        // Collecting Results
        //
        // Collect `tree` results from executors
        //
        //  - Receives processed 'tree' from [tree_rx] channel.
        //  - Collects all 'tree' results into 'all_tree_results'.
        //  - Aggregates 'all_tree_results' into 'ordered_tree_results'.
        //
        let mut all_tree_responses = Vec::new();
        for (index, mut tree_rx) in result_collectors {
            let executor_results = tokio::spawn(async move {
                let mut trees = Vec::new();
                while let Some(result) = tree_rx.recv().await {
                    trees.push(result);
                }
                (index, trees)
            });
            all_tree_responses.push(executor_results);
        }

        let all_tree_results = join_all(all_tree_responses).await;

        // Aggregate results from all workers in order
        let mut ordered_tree_results = vec![None; self.entry_csvs.len()];
        for result in all_tree_results {
            let (index, worker_results) = result.unwrap();
            let start = index * entries_per_executor;
            for (i, res) in worker_results.iter().enumerate() {
                ordered_tree_results[start + i] = Some(res.clone());
            }
        }

        // Terminate executors
        self.executor_spawner.terminate_executors().await;

        let all_merkle_sum_tree: Vec<MerkleSumTree<N_CURRENCIES, N_BYTES>> =
            ordered_tree_results.into_iter().flatten().collect();

        // Occur error if the number of mini_tree in 'all_merkle_sum_tree' is not equal to the number of entry_csvs.
        if all_merkle_sum_tree.len() != self.entry_csvs.len() {
            return Err("Mismatch in generated mini tree counts and given CSV counts".into());
        }

        AggregationMerkleSumTree::new(
            all_merkle_sum_tree,
            vec![
                Cryptocurrency {
                    name: "DUMMY".to_string(),
                    chain: "ETH".to_string(),
                };
                N_CURRENCIES
            ],
        )
    }
}
