use std::error::Error;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::{future::Future, pin::Pin};

use bollard::network::ListNetworksOptions;
use bollard::service::{ListServicesOptions, UpdateServiceOptions};
use tokio::sync::oneshot;

use crate::executor::utils::get_specs_from_compose;
use crate::executor::{Executor, ExecutorSpawner};

pub struct CloudSpawner {
    service_info: Option<(String, String)>,
    worker_counter: Arc<AtomicUsize>,
    worker_node_url: Vec<String>,
    default_port: i64,
}

/// CloudSpawner
///
/// Designed for cloud-based resources and Docker Swarm, CloudSpawner is optimized for scalability and high availability.
/// While functioning similarly to LocalSpawner, it extends its capabilities by initializing workers on remote machines, making it particularly suitable for Swarm network setups.
///
/// CloudSpawner can be utilized in two ways:
///
/// - Without `service_info`, CloudSpawner does not directly manage Worker instances.
///   In this mode, it does not control or interact with the Docker API for worker management.
///
/// - With `service_info`, CloudSpawner requires a `docker-compose` file. When provided with `service_info`,
///   it manages Docker services and networks, enabling dynamic scaling and orchestration of workers.
impl CloudSpawner {
    pub fn new(
        service_info: Option<(String, String)>, // If the user want to use docker-compose.yml for docker swarm
        worker_node_url: Vec<String>,
        default_port: i64,
    ) -> Self {
        assert!(!worker_node_url.is_empty(), "Worker node url is empty");
        CloudSpawner {
            service_info,
            worker_counter: Arc::new(AtomicUsize::new(0)),
            worker_node_url,
            default_port,
        }
    }

    async fn create_service(service_name: &str, compose_path: &str) -> Result<(), Box<dyn Error>> {
        let docker = bollard::Docker::connect_with_local_defaults().unwrap();

        // Retrieve network options and service spec from docker-compose.yml
        let (network_options, service_spec) =
            get_specs_from_compose(service_name, compose_path).unwrap();

        // Check network exist then create if not exist
        let list_network = docker
            .list_networks(None::<ListNetworksOptions<String>>)
            .await?;

        let mut found_target_network = false;
        list_network.iter().for_each(|network| {
            if service_name == *network.name.as_ref().unwrap() {
                found_target_network = true;
            }
        });

        if !found_target_network {
            match docker.create_network(network_options).await {
                Ok(result) => println!("Network created: {:?}", result),
                Err(error) => eprintln!("Error creating network: {}", error),
            }
        }

        // Checking service exist then create if not exist
        let services = docker
            .list_services(None::<ListServicesOptions<String>>)
            .await?;

        let mut found_exist_service = false;
        let mut service_version = 0;

        services.iter().for_each(|service| {
            let retrieved_service_spec = service
                .spec
                .as_ref()
                .ok_or::<Box<dyn Error>>("No spec in service on Docker".into())
                .unwrap();
            let retrieved_service_name = retrieved_service_spec
                .name
                .as_ref()
                .ok_or::<Box<dyn Error>>("No name in service.spec on Docker".into())
                .unwrap();

            if service_name == *retrieved_service_name {
                found_exist_service = true;

                // Update service version
                let retrieved_service_version = service
                    .version
                    .as_ref()
                    .ok_or::<Box<dyn Error>>("No version in service on Docker".into())
                    .unwrap();
                if let Some(exist_version) = retrieved_service_version.index {
                    service_version = exist_version + 1;
                }
            }
        });

        if !found_exist_service {
            docker.create_service(service_spec, None).await?;
            println!("Service {:?} created", service_name);
        } else {
            println!(
                "Service {:?} already exists, Will use the service",
                service_name
            );
            let update_service_options = UpdateServiceOptions {
                version: service_version,
                ..Default::default()
            };
            let update_response = docker
                .update_service(service_name, service_spec, update_service_options, None)
                .await?;
            update_response.warnings.iter().for_each(|warning| {
                println!("warning: {:?}", warning);
            });
        };
        Ok(())
    }
}

impl ExecutorSpawner for CloudSpawner {
    fn spawn_executor(&self) -> Pin<Box<dyn Future<Output = Executor> + Send>> {
        let (tx, rx) = oneshot::channel();

        let current_worker_counter = self.worker_counter.load(Ordering::SeqCst);

        // Create service if the worker counter is 0, which means no executor is spawned.
        if current_worker_counter == 0 && self.service_info.is_some() {
            let (service_name, compose_path) = self.service_info.clone().unwrap();
            tokio::spawn(async move {
                if let Err(e) = CloudSpawner::create_service(&service_name, &compose_path).await {
                    eprintln!("Error creating service: {}", e);
                } else {
                    // Sleep for 5 seconds to wait for the service to be ready
                    std::thread::sleep(std::time::Duration::from_secs(5));
                    let _ = tx.send(service_name.clone());
                    println!("Service {} created", service_name);
                }
            });
        }

        // The traffic is routed to the service by the swarm manager.
        // So, All executor can use the same exposed endpoint for distributing task to multiple workers.
        let port = self.default_port;
        let node_url = self.worker_node_url[current_worker_counter].clone();
        let worker_counter = self.worker_counter.clone();
        Box::pin(async move {
            if worker_counter.load(Ordering::SeqCst) == 0 {
                let _ = rx.await;
            }
            // Check if the URL already contains a port
            let has_port = node_url.split(':').last().unwrap().parse::<u16>().is_ok();

            // Append the port if it's not there
            let final_url = if has_port {
                node_url.clone()
            } else {
                format!("{}:{}", node_url, port)
            };
            worker_counter.fetch_add(1, Ordering::SeqCst);
            Executor::new(format!("http://{}", final_url), None)
        })
    }

    fn terminate_executors(&self) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        let service_info = self.service_info.clone();
        Box::pin(async move {
            if let Some((service_name, _)) = service_info.clone() {
                let docker = bollard::Docker::connect_with_local_defaults().unwrap();

                docker.delete_service(&service_name).await.unwrap();
                docker.remove_network(&service_name).await.unwrap();
            }
        })
    }
}
