use bollard::models::{
    EndpointSpec, NetworkAttachmentConfig, ServiceSpec, TaskSpec, TaskSpecContainerSpec,
    TaskSpecPlacement,
};
use bollard::service::{EndpointPortConfig, ListServicesOptions, UpdateServiceOptions};
use bollard::Docker;
use docker_compose_types::Compose;
use std::{future::Future, pin::Pin};

use crate::executor::{Executor, ExecutorSpawner};

// TODO: the CouldSpawner can control services on swarm networks using docker API.
pub struct CloudSpawner {
    docker: Docker,
    service_id: String,
    service_name: String,
    exposed_port: i64,
}

impl CloudSpawner {
    pub async fn new(
        service_name: &str,
        compose_path: &str,
        exposed_port: i64,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let file_payload = std::fs::read_to_string(compose_path).unwrap();
        let compose = match serde_yaml::from_str::<Compose>(&file_payload) {
            Ok(c) => c,
            Err(e) => panic!("Failed to parse docker-compose file: {}", e),
        };

        // Retrieve spec in docker-compose file
        let service = compose
            .services
            .0
            .get(service_name)
            .unwrap()
            .as_ref()
            .unwrap();
        let image_name = service.image.as_ref().unwrap();
        let replicas = service.deploy.as_ref().unwrap().replicas.unwrap();
        let placement = service.deploy.as_ref().unwrap().placement.as_ref().unwrap();

        let docker = bollard::Docker::connect_with_local_defaults().unwrap();

        // Checking service exist
        let services = docker
            .list_services(None::<ListServicesOptions<String>>)
            .await?;

        let mut found_exist_service = false;
        let mut service_id = String::new();
        services.iter().for_each(|service| {
            if service_name == service.spec.as_ref().unwrap().name.as_ref().unwrap() {
                found_exist_service = true;
                service_id = service.id.as_ref().unwrap().to_string();
            }
        });

        let service_spec = ServiceSpec {
            name: Some(String::from(service_name)),
            task_template: Some(TaskSpec {
                placement: Some(TaskSpecPlacement {
                    constraints: Some(placement.constraints.clone()),
                    ..Default::default()
                }),
                container_spec: Some(TaskSpecContainerSpec {
                    image: Some(image_name.to_string()),
                    ..Default::default()
                }),
                networks: Some(vec![NetworkAttachmentConfig {
                    target: Some("summa_mini_tree_net".to_string()),
                    ..Default::default()
                }]),
                ..Default::default()
            }),
            endpoint_spec: Some(EndpointSpec {
                ports: Some(vec![EndpointPortConfig {
                    target_port: Some(4000),
                    published_port: Some(exposed_port),
                    ..Default::default()
                }]),
                ..Default::default()
            }),
            ..Default::default()
        };

        if found_exist_service {
            println!(
                "Service {:?} already exists, Will use the service",
                service_name
            );
            let update_service_options = UpdateServiceOptions {
                version: 1234, // TODO: version control
                ..Default::default()
            };
            let update_response = docker
                .update_service(service_name, service_spec, update_service_options, None)
                .await?;
            update_response.warnings.iter().for_each(|warning| {
                println!("warning: {:?}", warning);
            });
        } else {
            // Use service_spec with bollard to create the service
            let service_response = docker.create_service(service_spec, None).await?;
            service_id = service_response.id.ok_or("Failed to get service ID")?;
            println!("Service {:?} created", service_name);
        }

        Ok(CloudSpawner {
            docker: Docker::connect_with_local_defaults().unwrap(),
            service_id,
            service_name: service_name.to_string(),
            exposed_port,
        })
    }
}

impl ExecutorSpawner for CloudSpawner {
    fn spawn_executor(&self) -> Pin<Box<dyn Future<Output = Executor> + Send>> {
        // The traffic is routed to the service by the swarm manager.
        // So, All executor can use the same exposed endpoint for distributing task to multiple workers.
        let endpoint = self.exposed_port;
        Box::pin(
            async move { Executor::new(format!("http://localhost:{}", endpoint), None) },
        )
    }

    fn terminate_executors(&self) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        Box::pin(async move {
            // Nothing to do
        })
    }
}
