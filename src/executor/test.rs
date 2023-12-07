#![allow(unused_imports)]
use futures::future;
use std::error::Error;

use bollard::models::TaskSpecContainerSpec;

use crate::executor::{spawner::ExecutorSpawner, utils::get_specs_from_compose, MockSpawner};
use crate::json_mst::JsonEntry;
use summa_backend::merkle_sum_tree::utils::parse_csv_to_entries;

#[test]
fn test_util_get_specs_from_compose() {
    let (network_options, service_spec) =
        get_specs_from_compose("mini_tree", "docker-compose.yml").unwrap();

    let service_name = "mini_tree";
    assert_eq!(network_options.name, service_name);
    assert_eq!(network_options.driver, "overlay");

    assert_eq!(service_spec.name.unwrap(), service_name);
    assert!(service_spec.mode.is_some());
    assert!(service_spec.task_template.is_some());
    assert!(service_spec.endpoint_spec.is_some());
    assert_eq!(
        service_spec.task_template.unwrap().container_spec.unwrap(),
        TaskSpecContainerSpec {
            image: Some("summadev/summa-aggregation-mini-tree:latest".to_string()),
            ..Default::default()
        }
    );
}

#[tokio::test]
async fn test_executor() -> Result<(), Box<dyn Error>> {
    let spawner = MockSpawner::new(None);
    let executor = spawner.spawn_executor().await;

    let (_, entries) = parse_csv_to_entries::<_, 2, 14>("csv/entry_16.csv").unwrap();
    let json_entries = entries
        .iter()
        .map(JsonEntry::from_entry)
        .collect::<Vec<JsonEntry>>();
    let merkle_sum_tree = executor.generate_tree::<2, 14>(json_entries).await.unwrap();

    spawner.terminate_executors().await;

    assert_eq!(merkle_sum_tree.index_of_username("dxGaEAii").unwrap(), 0);
    Ok(())
}

#[tokio::test]
async fn test_executor_block() -> Result<(), Box<dyn Error>> {
    let spawner = MockSpawner::new(None);
    let executor = spawner.spawn_executor().await;

    // Parse two csv files
    let (_, entries_1) = parse_csv_to_entries::<_, 2, 14>("csv/entry_16.csv").unwrap();
    let (_, entries_2) = parse_csv_to_entries::<_, 2, 14>("csv/entry_16.csv").unwrap();

    // Convert entries to json_entries
    let json_entries_1 = entries_1
        .iter()
        .map(JsonEntry::from_entry)
        .collect::<Vec<JsonEntry>>();
    let json_entries_2 = entries_2
        .iter()
        .map(JsonEntry::from_entry)
        .collect::<Vec<JsonEntry>>();

    let merkle_tree_1 = executor.generate_tree::<2, 14>(json_entries_1);
    let merkle_tree_2 = executor.generate_tree::<2, 14>(json_entries_2);

    let all_tree = future::join_all([merkle_tree_1, merkle_tree_2]).await;

    spawner.terminate_executors().await;

    assert_eq!(all_tree.len(), 2);

    Ok(())
}
