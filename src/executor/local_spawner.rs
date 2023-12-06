use bollard::{
    container::{Config, CreateContainerOptions, RemoveContainerOptions, StartContainerOptions},
    service::ContainerInspectResponse,
    Docker,
};
use std::{
    default::Default,
    env,
    error::Error,
    future::Future,
    pin::Pin,
    sync::atomic::{AtomicUsize, Ordering},
};
use tokio;
use tokio::sync::oneshot;

use crate::executor::{Executor, ExecutorSpawner};

pub struct LocalSpawner {
    docker: Docker,
    worker_counter: AtomicUsize,
    image_name: String,
    container_name: String,
}

impl LocalSpawner {
    pub fn new(image_name: String, container_name: String) -> Self {
        let docker = match env::var("DOCKER_HOST") {
            // Read `DOCKER_HOST` environment variable as default
            Ok(host) => Docker::connect_with_http_defaults()
                .unwrap_or_else(|_| panic!("Failed to connect to {} for using Docker", host)),
            _ => Docker::connect_with_local_defaults()
                .unwrap_or_else(|_| panic!("Failed to connect to Docker")),
        };

        LocalSpawner {
            docker,
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

        println!("docker-info: {:?}", docker.info().await?);

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

        println!("container_info: {:?}", container_info);

        Ok(container_info)
    }
}

impl ExecutorSpawner for LocalSpawner {
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
                LocalSpawner::create_container(docker_clone, image_name, container_name, id).await
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
        let worker_counter = self.worker_counter.load(Ordering::SeqCst);
        Box::pin(async move {
            // Remove the container
            let remove_options = RemoveContainerOptions {
                force: true, // Force the removal of the container
                ..Default::default()
            };

            for i in 0..worker_counter {
                let container_name = format!("{}_{}", container_name, i);
                if let Err(e) = docker_clone
                    .remove_container(&container_name, Some(remove_options))
                    .await
                {
                    eprintln!("Error removing container: {}", e);
                }
            }
        })
    }
}

#[cfg(feature = "docker")]
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_executor_spawner() {
        let spawner = LocalSpawner::new(
            "summadev/summa-aggregation-mini-tree:latest".to_string(),
            "executor_test".to_string(),
        );

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
