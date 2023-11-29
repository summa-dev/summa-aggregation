use axum::{
    routing::post,
    Router,
};
use std::{
    future::Future,
    net::SocketAddr,
    pin::Pin,
    str::FromStr,
    sync::atomic::{AtomicUsize, Ordering},
};
use tokio;
use tokio::sync::oneshot;

use crate::executor::{Executor, ExecutorSpawner};
use crate::mini_tree_generator::create_mst;

pub struct MockSpawner {
    urls: Option<Vec<String>>,
    worker_counter: AtomicUsize,
}

impl MockSpawner {
    pub fn new(urls: Option<Vec<String>>) -> Self {
        MockSpawner {
            urls,
            worker_counter: AtomicUsize::new(0),
        }
    }
}

impl ExecutorSpawner for MockSpawner {
    fn spawn_executor(&self) -> Pin<Box<dyn Future<Output = Executor> + Send>> {
        let (tx, rx) = oneshot::channel();

        let id = self.worker_counter.fetch_add(1, Ordering::SeqCst);

        // If urls is not None, use the urls to spawn executors
        if self.urls.is_some() && self.urls.as_ref().unwrap().len() > id {
            let url = self.urls.as_ref().unwrap()[id].clone();
            let _ = tx.send(SocketAddr::from_str(&url).unwrap());

            return Box::pin(async move {
                let url = rx.await.expect("Failed to receive worker URL");
                let worker_url = format!("http://{}", url);
                Executor::new(worker_url, None)
            });
        }

        // if there is no url or already used all urls, spawn a new executor
        tokio::spawn(async move {
            let app = Router::new()
                .route("/", post(create_mst));

            let addr = SocketAddr::from(([0, 0, 0, 0], 4000 + id as u16));

            // send worker url to rx
            let _ = tx.send(addr.clone());

            // Start the server
            axum::Server::bind(&addr).serve(app.into_make_service()).await.unwrap();
        });

        // Return a Future that resolves to Executor
        Box::pin(async move {
            // load currnet worker counter
            let url = rx.await.expect("Failed to receive worker URL");
            let worker_url = format!("http://{}", url);
            Executor::new(worker_url, None)
        })
    }

    fn terminate_executors(&self) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        Box::pin(async move {
            // Nothing to do if no executors are running
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_new_urls() {
        let spawner = MockSpawner::new(None);

        // Spawn 2 executors
        let executor_1 = spawner.spawn_executor().await;
        let executor_2 = spawner.spawn_executor().await;

        // Sleep 2 seconds for the container to be ready
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        assert!(!executor_1.get_url().is_empty());
        assert!(!executor_2.get_url().is_empty());
    }

    #[tokio::test]
    async fn test_with_given_url() {
        let urls = vec!["127.0.0.1:7878".to_string()];
        let spawner = MockSpawner::new(Some(urls));

        // Spawn 2 executors
        let executor_1 = spawner.spawn_executor().await;
        let executor_2 = spawner.spawn_executor().await;

        assert_eq!(executor_1.get_url(), "http://127.0.0.1:7878");
        assert_eq!(executor_2.get_url(), "http://0.0.0.0:4001");
    }
}
