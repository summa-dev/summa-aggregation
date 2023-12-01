use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;

use bollard::models::{
    NetworkAttachmentConfig, ServiceSpec, ServiceSpecMode, ServiceSpecModeReplicated, TaskSpec,
    TaskSpecContainerSpec, TaskSpecPlacement,
};
use bollard::network::CreateNetworkOptions;
use bollard::service::{EndpointPortConfig, EndpointPortConfigPublishModeEnum, EndpointSpec};

#[derive(Debug, Serialize, Deserialize)]
pub struct DockerCompose {
    pub version: String,
    pub services: HashMap<String, Service>,
    pub networks: Option<HashMap<String, Network>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Service {
    pub image: String,
    pub ports: Option<Vec<Port>>,
    pub deploy: Option<Deploy>,
    pub networks: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Port {
    pub target: i64,
    pub published: i64,
    pub published_mode: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Deploy {
    pub mode: Option<String>,
    pub placement: Option<Placement>,
    pub replicas: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Placement {
    pub constraints: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Network {
    driver: Option<String>,
}

// This helper function return `CreateNetworkOptions` and `ServiceSpec` from `docker-compose.yml`
pub fn get_specs_from_compose(
    service_name: &str,
    file_path: &str,
) -> Result<(CreateNetworkOptions<String>, ServiceSpec), Box<dyn Error>> {
    let file_content = std::fs::read_to_string(file_path).expect("Unable to read file");
    let compose = serde_yaml::from_str::<DockerCompose>(&file_content)?;

    // Declare docker client & default labels
    let mut labels = HashMap::new();
    labels.insert(
        String::from("Summa"),
        String::from("Dummy key / value for bollard"),
    );

    // Retrieve network options from docker-compose.yml
    let mut network_options = CreateNetworkOptions::<String>::default();
    compose
        .networks
        .ok_or("There is no network configuration")
        .unwrap()
        .iter()
        .for_each(|(network_name, network)| {
            if network_name == service_name {
                network_options.name = network_name.to_string();
                network_options.driver = network.driver.clone().unwrap_or("overlay".to_string());
                network_options.labels = labels.clone();
            }
        });

    if network_options.name.is_empty() {
        return Err(format!(
            "Network name is empty, It may not exist network name: '{}' in docker-compose file",
            service_name
        )
        .into());
    }

    // Retrieve service spec from docker-compose.yml
    let service_spec = match compose.services.get(service_name) {
        Some(service) => {
            // Parse these variables from docker-compose.yml
            let ports = service.ports.as_ref().ok_or("There is no 'ports' field")?;
            let endpoint_port_config = ports
                .iter()
                .map(|port| EndpointPortConfig {
                    target_port: Some(port.target),
                    published_port: Some(port.published),
                    publish_mode: Some(EndpointPortConfigPublishModeEnum::INGRESS),
                    ..Default::default()
                })
                .collect::<Vec<EndpointPortConfig>>();

            let deploy = service
                .deploy
                .as_ref()
                .ok_or("There is no 'deploy' field")?;
            let parsed_replicas = deploy
                .replicas
                .ok_or("There is no 'replicas' under 'deploy' field")?;
            let parsed_contraint = deploy
                .placement
                .as_ref()
                .ok_or("There is no 'placement' field")?
                .constraints
                .as_ref()
                .ok_or("There is no 'constraints' under 'placement' field")?;

            ServiceSpec {
                name: Some(String::from(service_name)),
                mode: Some(ServiceSpecMode {
                    replicated: Some(ServiceSpecModeReplicated {
                        replicas: Some(parsed_replicas),
                    }),
                    ..Default::default()
                }),
                task_template: Some(TaskSpec {
                    placement: Some(TaskSpecPlacement {
                        constraints: Some(parsed_contraint.to_owned()),
                        ..Default::default()
                    }),
                    container_spec: Some(TaskSpecContainerSpec {
                        image: Some(service.image.clone()),
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
                endpoint_spec: Some(EndpointSpec {
                    mode: None,
                    ports: Some(endpoint_port_config),
                }),
                networks: Some(vec![NetworkAttachmentConfig {
                    target: Some(service_name.to_string()),
                    ..Default::default()
                }]),
                ..Default::default()
            }
        }
        None => {
            return Err(format!(
                "Service name: '{}' not found in docker-compose file",
                service_name
            )
            .into())
        }
    };

    Ok((network_options, service_spec))
}
