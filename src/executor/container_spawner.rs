use bollard::{
    container::{
        self, Config, CreateContainerOptions, RemoveContainerOptions, StartContainerOptions,
    },
    service::ContainerInspectResponse,
    Docker,
};
use std::{
    default::Default,
    error::Error,
    future::Future,
    pin::Pin,
    sync::atomic::{AtomicUsize, Ordering},
};
use tokio;
use tokio::sync::oneshot;

use crate::executor::{Executor, ExecutorSpawner};

pub struct ContainerSpawner {
    docker: Docker,
    worker_counter: AtomicUsize,
    image_name: String,
    container_name: String,
}

impl ContainerSpawner {
    pub fn new(image_name: String, container_name: String) -> Self {
        ContainerSpawner {
            docker: Docker::connect_with_local_defaults().unwrap(),
            worker_counter: AtomicUsize::new(0),
            image_name,
            container_name,
        }
    }

    // Create a Docker instance connected to the local Docker daemon.
    pub async fn create_container(
        docker: Docker,
        image_name: String,
        container_name: String,
        id: usize,
    ) -> Result<ContainerInspectResponse, Box<dyn Error>> {
        let container_name = format!("{}_{}", container_name, id);

        let config = Config {
            image: Some(image_name),
            ..Default::default()
        };

        // Create the container.
        let create_container_options = CreateContainerOptions {
            name: container_name.clone(),
            platform: None,
        };

        docker
            .create_container(Some(create_container_options), config.clone())
            .await?;

        docker
            .start_container(
                &container_name.clone(),
                None::<StartContainerOptions<String>>,
            )
            .await?;

        let container_info: ContainerInspectResponse =
            docker.inspect_container(&container_name, None).await?;

        Ok(container_info)
    }
}

impl ExecutorSpawner for ContainerSpawner {
    fn spawn_executor(&self) -> Pin<Box<dyn Future<Output = Executor> + Send>> {
        // Using channel that onetime use, `oneshot`, to send container information
        let (tx, rx) = oneshot::channel();

        // These variables has to be cloned because it is moved into the async block
        let docker_clone = self.docker.clone();
        let image_name = self.image_name.clone();
        let container_name = self.container_name.clone();
        let id = self.worker_counter.fetch_add(1, Ordering::SeqCst);
        tokio::spawn(async move {
            if let Ok(container_info) =
                ContainerSpawner::create_container(docker_clone, image_name, container_name, id)
                    .await
            {
                let _ = tx.send(container_info);
            }
        });

        // Return a Future that resolves to Executor
        Box::pin(async move {
            let container_info = rx.await.expect("Failed to receive worker URL");
            let worker_url = format!(
                "http://{}:4000", // this port is not exposed to the host
                container_info.network_settings.unwrap().ip_address.unwrap()
            );
            Executor::new(worker_url, container_info.name)
        })
    }

    fn terminate_executors(&self) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        let docker_clone = self.docker.clone();

        let container_name = self.container_name.clone();
        let worker_counter = self.worker_counter.load(Ordering::SeqCst).clone();
        Box::pin(async move {
            // Remove the container
            let remove_options = RemoveContainerOptions {
                force: true, // Force the removal of the container
                ..Default::default()
            };

            for i in 0..worker_counter {
                let container_name = format!("{}_{}", container_name, i);
                if let Err(e) = docker_clone
                    .remove_container(&container_name, Some(remove_options.clone()))
                    .await
                {
                    eprintln!("Error removing container: {}", e);
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    #[tokio::test]
    async fn test_executor_spawner() {
        let spawner = ContainerSpawner {
            docker: Docker::connect_with_local_defaults().unwrap(),
            worker_counter: AtomicUsize::new(0),
            image_name: "summa-aggregation".to_string(), // Should exist on local registry
            container_name: "mini_tree_generator".to_string(),
        };

        // Spawn 2 executors
        let executor_1 = spawner.spawn_executor().await;
        let executor_2 = spawner.spawn_executor().await;

        // Sleep 2 seconds for the container to be ready
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        assert!(!executor_1.get_url().is_empty());
        assert!(!executor_2.get_url().is_empty());

        // Teardown
        spawner.terminate_executors().await;
    }
}
