mod cloud_spawner;
mod local_spawner;
mod mock_spawner;
mod spawner;
mod test;

pub use cloud_spawner::CouldSpawner;
pub use local_spawner::LocalSpawner;
pub use mock_spawner::MockSpawner;
pub use spawner::ExecutorSpawner;

use reqwest::Client;
use std::error::Error;

use crate::json_mst::{JsonEntry, JsonMerkleSumTree};
use summa_backend::merkle_sum_tree::MerkleSumTree;

#[derive(Clone)]
pub struct Executor {
    client: Client,
    url: String,
    id: Option<String>,
}

impl Executor {
    pub fn new(url: String, id: Option<String>) -> Self {
        Executor {
            client: Client::new(),
            url,
            id,
        }
    }

    pub fn get_url(&self) -> String {
        self.url.clone()
    }

    pub fn get_name(&self) -> Option<String> {
        self.id.clone()
    }

    pub async fn generate_tree<const N_ASSETS: usize, const N_BYTES: usize>(
        &self,
        json_entries: Vec<JsonEntry>,
    ) -> Result<MerkleSumTree<N_ASSETS, N_BYTES>, Box<dyn Error + Send>> {
        let response = self
            .client
            .post(&self.url)
            .json(&json_entries)
            .send()
            .await
            .map_err(|err| Box::new(err) as Box<dyn Error + Send>)?;

        let json_tree = response
            .json::<JsonMerkleSumTree>()
            .await
            .map_err(|err| Box::new(err) as Box<dyn Error + Send>)?;

        let tree = json_tree.to_mst();

        Ok(tree)
    }
}
