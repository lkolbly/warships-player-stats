use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::time::{sleep, Duration};

#[macro_use]
use tracing::*;

use crate::scraper::WowsClient;
use crate::wows_data::ShipInfo;

#[derive(Clone)]
pub struct ShipDb {
    ships: Arc<Mutex<HashMap<u64, ShipInfo>>>,
}

impl ShipDb {
    pub fn new() -> Self {
        let ships = Arc::new(Mutex::new(HashMap::new()));
        Self { ships }
    }

    pub fn get_ship_info(&self, shipid: u64) -> Option<ShipInfo> {
        let ships = self.ships.lock().unwrap();
        ships.get(&shipid).map(|x| (*x).clone())
    }

    pub fn get_all_info(&self) -> HashMap<u64, ShipInfo> {
        let ships = self.ships.lock().unwrap();
        ships.clone()
    }

    pub async fn update_loop(self, client: WowsClient) {
        loop {
            match client.enumerate_ships().await {
                Ok(data) => {
                    info!("Loaded ship database, contains {} ships", data.len());
                    let mut ships = self.ships.lock().unwrap();
                    *ships = data;
                }
                Err(e) => {
                    eprintln!("Got error! {:?}", e);
                }
            }

            sleep(Duration::from_millis(24 * 3600 * 1000)).await;
        }
    }
}
