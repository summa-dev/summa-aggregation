use bollard::{
    container::{Config, CreateContainerOptions, RemoveContainerOptions, StartContainerOptions},
    models::{HostConfig, PortBinding},
    service::ContainerInspectResponse,
    Docker,
};
use std::{
    collections::HashMap,
    default::Default,
    env,
    error::Error,
    future::Future,
    net::{SocketAddr, TcpListener, IpAddr},
    pin::Pin,
    sync::atomic::{AtomicUsize, Ordering}, str::FromStr,
};
use tokio;
use tokio::sync::oneshot;

use crate::executor::{Executor, ExecutorSpawner};

/// LocalSpawner
///
/// The LocalSpawner is to use cases closer to actual deployment. It enables the initialization of Executors
/// and Workers within a local Docker environment. This spawner is ideal for development and testing phases,
/// where simplicity and direct control over the containers are beneficial.
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

    fn find_unused_port() -> Result<u16, std::io::Error> {
        // Bind to address with port 0.
        // The OS will assign an available ephemeral port.
        let listener = TcpListener::bind("127.0.0.1:0")?;

        // Retrieve the assigned port.
        match listener.local_addr() {
            Ok(SocketAddr::V4(addr)) => Ok(addr.port()),
            Ok(SocketAddr::V6(addr)) => Ok(addr.port()),
            Err(e) => Err(e),
        }
    }

    // Create a Docker instance connected to the local Docker daemon.
    pub async fn create_container(
        docker: Docker,
        image_name: String,
        container_name: String,
        id: usize,
        desirable_port: u16,
    ) -> Result<ContainerInspectResponse, Box<dyn Error>> {
        let container_name = format!("{}_{}", container_name, id);

        // Define port mapping (container_port -> host_port)
        let port_bindings = {
            let mut port_bindings = HashMap::new();
            port_bindings.insert(
                "4000/tcp".to_string(), // Container port
                Some(vec![PortBinding {
                    host_ip: Some(IpAddr::from_str("127.0.0.1").unwrap().to_string()), // Host IP
                    host_port: Some(desirable_port.to_string()),                       // Host port
                }]),
            );
            port_bindings
        };

        let config = Config {
            image: Some(image_name),
            exposed_ports: Some(HashMap::from([("4000/tcp".to_string(), HashMap::<(), ()>::new())])), // Expose the container port
            host_config: Some(HostConfig {
                port_bindings: Some(port_bindings),
                ..Default::default()
            }),
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

impl ExecutorSpawner for LocalSpawner {
    fn spawn_executor(&self) -> Pin<Box<dyn Future<Output = Executor> + Send>> {
        // Using channel that onetime use, `oneshot`, to send container information
        let (tx, rx) = oneshot::channel();

        // These variables has to be cloned because these are moved into the async block
        let docker_clone = self.docker.clone();
        let image_name = self.image_name.clone();
        let container_name = self.container_name.clone();
        let id = self.worker_counter.fetch_add(1, Ordering::SeqCst);
        tokio::spawn(async move {
            let desirable_port = LocalSpawner::find_unused_port().unwrap_or_default();
            let res = LocalSpawner::create_container(
                docker_clone,
                image_name,
                container_name,
                id,
                desirable_port,
            )
            .await;
            match res {
                Ok(container_info) => {
                    // the desirable_port is the port that is exposed to the host
                    let _ = tx.send((desirable_port, container_info));
                }
                Err(e) => {
                    eprintln!("Error creating container: {}", e);
                }
            }
        });

        // Return a Future that resolves to Executor
        Box::pin(async move {
            // the container_info also has exposed port as 'host_port` field but it looks ugly to use it 
            let (exposed_port, container_info) = rx.await.expect("Failed to receive worker URL");
            let worker_url = format!(
                "http://127.0.0.1:{}", // This port is exposed to the host
                exposed_port
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
                let container_name_with_id = format!("{}_{}", container_name, i);
                if let Err(e) = docker_clone
                    .remove_container(&container_name_with_id, Some(remove_options))
                    .await
                {
                    eprintln!("Error removing container: {}", e);
                }
            }
        })
    }
}

// #[cfg(feature = "docker")]
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
