use std::{future::Future, pin::Pin};

use crate::executor::Executor;

pub trait ExecutorSpawner {
    /// Spawns an executor asynchronously.
    ///
    /// This method initializes an Executor and returns a Future that resolves to the Executor.
    ///
    /// To achieve this asynchronously (outside of an async trait function), we use a one-time channel (`oneshot`) to deliver the variables to the Future.
    ///
    /// Internally, it performs the following codelines:
    ///
    /// 1. Uses a `oneshot` channel for sending the variables from the spawned async task.
    ///    ```
    ///     let (tx, rx) = oneshot::channel();
    ///    ```
    /// 2. Clones necessary variables (url, name and so on) to move them into the async block.
    ///    ```
    ///     let url = self.url.clone();
    ///    ```
    /// 3. Spawns an asynchronous task (`tokio::spawn`) that asynchronously creates a worker and sends back its information.
    ///    ```
    ///     tokio::spawn(async move {
    ///     if let Ok(worker_info) =
    ///         Spawner::create_worker(url).await
    ///     {
    ///         let _ = tx.send(worker_info);
    ///     }
    ///    });
    ///     Note that, the `create_worker` is typically declared in the `Spawner` struct that has `ExecutorSpawner` trait.
    /// 4. Returns a Future that, upon completion, provides an Executor connected to the newly spawned worker.
    ///    ```
    ///    Box::pin(async move {
    ///             let url = rx.await.expect("Failed to receive worker URL");
    ///             Executor::new(url, None);
    ///    });
    ///   ```
    ///
    /// Returns:
    /// - `Pin<Box<dyn Future<Output = Executor> + Send>>`: A Future that, when awaited, yields an Executor instance and spawns a worker.
    ///
    fn spawn_executor(&self) -> Pin<Box<dyn Future<Output = Executor> + Send>>;

    /// Terminates all spawned executors (and/or workers) asynchronously.
    ///
    /// This method is responsible for gracefully shutting down all active executors (and/or workers) by calling
    /// To do this, the `Spawner` may needs some fields for storing some accessing point to the workers, which are spawned with the executors.
    /// For deliver variables to Future results, use a channel like the pattern at `spawn_executor`
    ///
    /// The termination process typically involves:
    /// - Iterating through all active Executors and workers.
    /// - Invoking kind of `shutdown` on each executors and workers to initiate their shutdown.
    /// - Awaiting the completion of all shutdown operations.
    ///
    /// Returns:
    /// - `Pin<Box<dyn Future<Output = ()> + Send>>`: A Future that, when awaited, indicates that all executors (and/or workers) have been terminated.
    fn terminate_executors(&self) -> Pin<Box<dyn Future<Output = ()> + Send>>;
}
