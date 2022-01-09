use itertools::Itertools;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use tokio::time::{sleep, Duration};

#[macro_use]
use tracing::*;

use crate::scraper::WowsClient;
use crate::wows_data::{DetailedModuleInfo, ShipInfo};

#[derive(Clone)]
pub struct ShipDb {
    ships: Arc<Mutex<HashMap<u64, ShipInfo>>>,
    modules: Arc<Mutex<HashMap<u64, DetailedModuleInfo>>>,
}

impl ShipDb {
    pub fn new() -> Self {
        let ships = Arc::new(Mutex::new(HashMap::new()));
        let modules = Arc::new(Mutex::new(HashMap::new()));
        Self { ships, modules }
    }

    pub fn get_ship_info(&self, shipid: u64) -> Option<ShipInfo> {
        let ships = self.ships.lock().unwrap();
        ships.get(&shipid).map(|x| (*x).clone())
    }

    pub fn get_all_info(&self) -> HashMap<u64, ShipInfo> {
        let ships = self.ships.lock().unwrap();
        ships.clone()
    }

    pub fn get_modules(&self) -> HashMap<u64, DetailedModuleInfo> {
        let modules = self.modules.lock().unwrap();
        modules.clone()
    }

    async fn update_modules(&mut self, client: &WowsClient) {
        let new_modules = {
            let moduleids = {
                let ships = self.ships.lock().unwrap();

                let mut moduleids = HashSet::new();
                for (_, ship) in ships.iter() {
                    for (_, module) in ship.modules_tree.iter() {
                        moduleids.insert(module.module_id);
                    }
                }
                moduleids
            };

            info!("Downloading {} modules...", moduleids.len());

            let mut queries = vec![];
            for moduleids in &moduleids.iter().chunks(100) {
                let chunk: Vec<_> = moduleids.map(|x| *x).collect();
                queries.push(chunk);
            }

            let mut modules = HashMap::new();
            for chunk in queries.iter() {
                for (id, module) in client
                    .get_module_info(chunk.as_slice())
                    .await
                    .unwrap()
                    .iter()
                {
                    modules.insert(*id, module.clone());
                }
            }
            modules
        };

        let mut modules = self.modules.lock().unwrap();
        *modules = new_modules;
    }

    pub async fn update_loop(mut self, client: WowsClient) {
        loop {
            info!("Updating ship DB");
            match client.enumerate_ships().await {
                Ok(data) => {
                    info!("Loaded ship database, contains {} ships", data.len());
                    let mut ships = self.ships.lock().unwrap();
                    *ships = data;
                }
                Err(e) => {
                    error!("Error enumerating ships: {:?}", e);
                }
            }

            info!("Loading modules...");
            self.update_modules(&client).await;
            info!("Modules updated successfully");

            sleep(Duration::from_millis(24 * 3600 * 1000)).await;
        }
    }

    pub fn enumerate_ships(&self) -> Vec<u64> {
        let ships = self.ships.lock().unwrap();
        ships.iter().map(|(k, _)| *k).collect()
    }
}
