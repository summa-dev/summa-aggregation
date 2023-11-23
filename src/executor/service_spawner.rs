use bollard::Docker;
use std::{
    future::Future,
    pin::Pin,
    sync::atomic::{AtomicUsize, Ordering},
};

use crate::executor::{Executor, ExecutorSpawner};

// TODO: the ServiceSpawner can control services on swarm networks using docker API.
pub struct ServiceSpawner {
    docker: Docker,
    request_counter: AtomicUsize,
    starting_port: u16,
    service_name: String,
}

impl ServiceSpawner {
    pub fn new(service_name: String, starting_port: u16) -> Self {
        ServiceSpawner {
            docker: Docker::connect_with_local_defaults().unwrap(),
            request_counter: AtomicUsize::new(0),
            starting_port,
            service_name,
        }
    }
}

impl ExecutorSpawner for ServiceSpawner {
    fn spawn_executor(&self) -> Pin<Box<dyn Future<Output = Executor> + Send>> {
        // Return a Future that resolves to Executor
        let worker_port =
            self.starting_port + self.request_counter.fetch_add(1, Ordering::SeqCst) as u16;

        Box::pin(async move {
            let worker_url = format!("http://localhost:{}", worker_port);
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
    use std::sync::atomic::AtomicUsize;

    #[tokio::test]
    async fn test_service_spawner() {
        let spawner = ServiceSpawner {
            docker: Docker::connect_with_local_defaults().unwrap(),
            request_counter: AtomicUsize::new(0),
            starting_port: 4000,
            service_name: "test_service".to_string(),
        };

        // Spawn 2 executors
        let executor_1 = spawner.spawn_executor().await;
        let executor_2 = spawner.spawn_executor().await;

        assert_eq!("http://localhost:4000", executor_1.get_url());
        assert_eq!("http://localhost:4001", executor_2.get_url());
    }
}
