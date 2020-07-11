use flate2::read::{GzDecoder, GzEncoder};
use flate2::Compression;
use futures::stream::FuturesUnordered;
use futures::stream::StreamExt;
use itertools::*;
use std::collections::HashMap;
use std::convert::TryInto;
use std::io::BufReader;
use std::io::Read;
use std::sync::{Arc, Mutex};
use std::time;
use std::time::Instant;
use tokio::fs::File;
use tokio::prelude::*;

use crate::error::Error;
use crate::progress_logger::ProgressLogger;
use crate::scraper::WowsClient;
use crate::statistics::*;
use crate::wows_data::*;

pub struct Database {
    //client: WowsClient,
    ships: HashMap<u64, ShipInfo>,
    stats: HashMap<u64, ShipStatsHistogram>,
    player_list: HashMap<String, u64>,
    player_data: Arc<sled::Db>,
}

impl Database {
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>, Error> {
        let mut gz = GzEncoder::new(data, Compression::fast());
        let mut body = vec![];
        gz.read_to_end(&mut body)?;
        Ok(body)
    }

    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>, Error> {
        let mut gz = GzDecoder::new(&data[..]);

        let mut body = vec![];
        gz.read_to_end(&mut body)?;
        Ok(body)
    }

    pub async fn new() -> Result<Database, Error> {
        let ships: HashMap<u64, ShipInfo> = {
            let mut f = File::open("ships.bin").await?;
            let mut v = vec![];
            f.read_to_end(&mut v).await?;
            bincode::deserialize(&v)?
        };
        let player_list: Vec<PlayerRecord> = {
            let mut encoded_players = vec![];
            File::open("playerlist.bin")
                .await?
                .read_to_end(&mut encoded_players)
                .await?;
            bincode::deserialize(&encoded_players)?
        };
        let mut player_lookup = HashMap::new();
        for player in player_list.iter() {
            player_lookup.insert(player.nickname.to_lowercase(), player.account_id);
        }
        let ship_stats_hist = calculate_aggregate_stats(player_list.len()).await.unwrap();
        Ok(Database {
            ships: ships,
            player_list: player_lookup,
            stats: ship_stats_hist,
            player_data: Arc::new(
                sled::open("player_data.sled").expect("Unable to create sled database!"),
            ),
        })
    }

    pub fn get_user(&self, username: &str) -> Option<u64> {
        match self.player_list.get(username) {
            Some(x) => Some(*x),
            None => None,
        }
    }

    pub fn get_ship_stats(&self, ship_id: u64) -> Option<(&ShipInfo, &ShipStatsHistogram)> {
        match (self.ships.get(&ship_id), self.stats.get(&ship_id)) {
            (Some(a), Some(b)) => Some((a, b)),
            _ => None,
        }
    }

    pub fn get_detailed_stats(&self, player_id: u64) -> Result<Vec<DetailedStatTypes>, Error> {
        let player_id = format!("detailed_stats:{}", player_id);
        let body = match self.player_data.get(player_id)? {
            Some(body) => body,
            _ => {
                return Err(Error::CacheEntryNotFound);
            }
        };
        let contents = self.decompress(&body)?;
        let contents = std::str::from_utf8(contents.as_ref())?;
        let data: Vec<DetailedStatTypes> = serde_json::from_str(contents)?;
        Ok(data)
    }

    pub fn update_detailed_stats(
        &self,
        player_id: u64,
        stats: &Vec<DetailedStatTypes>,
    ) -> Result<(), Error> {
        let player_id = format!("detailed_stats:{}", player_id);
        self.player_data
            .insert(
                &player_id,
                self.compress(serde_json::to_string(stats)?.as_bytes())?,
            )
            .expect(&format!(
                "Error adding player_id='{}' to the cache",
                player_id
            ));
        Ok(())
    }
}

pub async fn database_update_loop(api_key: &str, database: Arc<Mutex<Database>>) {
    // Keep a few things updated:
    // - The list of players
    // - Each of those player's detailed stats
    // - The list of boats
    println!("Starting DB update loop...");
    let client = WowsClient::new(api_key);
    loop {
        let start = Instant::now();

        if let Err(e) = update(&client, database.clone()).await {
            println!("Encountered error updating database: {:?}", e);
        }

        println!("Finished database update in {}s", start.elapsed().as_secs());
        let period = 7 * 3600 * 24; // 1 week
        if start.elapsed().as_secs() < period {
            std::thread::sleep(time::Duration::from_secs(
                period - start.elapsed().as_secs(),
            ));
        }
    }
}

async fn update(client: &WowsClient, database: Arc<Mutex<Database>>) -> Result<(), Error> {
    // Refresh the player list
    let new_player_list = fetch_player_list(&client).await;
    let mut player_list = {
        let database = database.lock().unwrap();
        database.player_list.clone()
    };
    let mut num_new_players = 0;
    for (k, v) in new_player_list.iter() {
        if !player_list.contains_key(k) {
            num_new_players += 1;
            player_list.insert(k.to_string(), *v);
        }
    }

    println!("Found {} new players! Saving to file", num_new_players);
    let res: Vec<u8> = bincode::serialize(&player_list).unwrap();
    {
        let mut f = File::create("playerlist.bin.new")
            .await
            .expect("Could not create file");
        f.write_all(&res)
            .await
            .expect("Could not populate playerlist file");
    }
    std::fs::rename("playerlist.bin.new", "playerlist.bin")
        .expect("Could not move playerlist file");

    // Get more detailed stats on each one
    fetch_player_data(&client, &player_list, database.clone()).await?;

    // Compute the new aggregate stats
    let aggregate_stats = calculate_aggregate_stats(player_list.len()).await?;

    // Grab the new ships
    let ships = update_shiplist(&client).await?;

    // Update the database with our learnings
    {
        let mut database = database.lock().unwrap();
        database.player_list = player_list;
        database.stats = aggregate_stats;
        database.ships = ships;
    }

    Ok(())
}

async fn update_shiplist(client: &WowsClient) -> Result<HashMap<u64, ShipInfo>, Error> {
    let ships = client.enumerate_ships().await?;
    //println!("Dumping full ship list...");
    let res: Vec<u8> = bincode::serialize(&ships).unwrap();
    {
        let mut f = File::create("ships.bin.new").await?;
        f.write_all(&res).await?;
    }
    std::fs::rename("ships.bin.new", "ships.bin").expect("Could not move ships file");
    Ok(ships)
}

async fn calculate_aggregate_stats(
    nplayers: usize,
) -> Result<HashMap<u64, ShipStatsHistogram>, Error> {
    println!("calculate_aggregate_stats({} players) called", nplayers);
    let f = std::fs::File::open("detailed_stats.bin")?;
    let mut f = BufReader::new(f);
    let mut ship_stats_hist = HashMap::new();
    loop {
        let mut buffer = [0; 4];
        match f.read_exact(&mut buffer[..]) {
            Ok(_) => {}
            Err(_) => {
                // We hit the end of the file
                break;
            }
        }
        let length = u32::from_le_bytes(buffer);
        let mut buffer = vec![];
        buffer.resize(length.try_into().unwrap(), 0);
        match f.read_exact(&mut buffer[..]) {
            Ok(_) => {}
            Err(e) => {
                println!("Read error {:?}", e);
                break;
            }
        }

        let stats: HashMap<String, Option<Vec<DetailedStatTypes>>> =
            match bincode::deserialize(&buffer) {
                Ok(x) => x,
                Err(_) => {
                    println!("Error bindecoding");
                    break;
                }
            };
        for (_player_id, stats) in stats.iter() {
            match stats {
                Some(stats) => {
                    for ship_stats in stats.iter() {
                        if ship_stats.pvp.wins + ship_stats.pvp.losses < 10 {
                            continue;
                        }
                        if !ship_stats_hist.contains_key(&ship_stats.ship_id) {
                            ship_stats_hist.insert(ship_stats.ship_id, ShipStatsHistogram::new());
                        }
                        ship_stats_hist
                            .get_mut(&ship_stats.ship_id)
                            .unwrap()
                            .increment(&AveragedShipStats::calculate(&ship_stats.pvp))
                            .unwrap();
                    }
                }
                None => {}
            }
        }
    }
    Ok(ship_stats_hist)
}

/// Due to the size of this dataset, we write it out to a file
async fn fetch_player_data(
    client: &WowsClient,
    player_list: &HashMap<String, u64>,
    database: Arc<Mutex<Database>>,
) -> Result<(), Error> {
    println!("fetch_player_data({} players) called", player_list.len());
    let mut async_tasks = FuturesUnordered::new();
    let mut player_stream =
        futures::stream::iter(player_list.iter().map(|(_name, account_id)| account_id)).fuse();
    let mut f = File::create("detailed_stats.bin.new").await?;
    let mut logger = ProgressLogger::new_with_target("fetch_player_data", player_list.len());
    loop {
        tokio::select! {
            Some(account_id) = player_stream.next(), if async_tasks.len() < 100 => {
                let client = client.fork();
                async_tasks.push(async move {
                    (
                        account_id,
                        client.get_detailed_stats(*account_id).await,
                    )
                });
            }
            Some(stats) = async_tasks.next(), if async_tasks.len() > 0 => {
                match stats {
                    (account_id, Ok(stats)) => {
                        // Stick these in a list or database or something
                        logger.increment(1);

                        let serialized = bincode::serialize(&stats)?;
                        let length: u32 = serialized.len().try_into().unwrap();
                        f.write_all(&length.to_le_bytes()).await?;
                        f.write_all(&serialized).await?;

                        match stats.iter().next() {
                            Some((_player_id, player)) => {
                                match player {
                                    Some(x) => {
                                        let database = database.lock().unwrap();
                                        database.update_detailed_stats(*account_id, x)?;
                                    }
                                    None => {
                                    }
                                };
                            },
                            None => {
                                //
                            }
                        }
                    }
                    (account_id, Err(e)) => {
                        println!("Something went wrong retrieving player {}! Error: {:?}", account_id, e);
                    }
                }
            }
            else => break,
        }
    }
    std::fs::rename("detailed_stats.bin.new", "detailed_stats.bin")
        .expect("Could not move detailed_stats file!");
    Ok(())
}

async fn fetch_player_list(client: &WowsClient) -> HashMap<String, u64> {
    println!("fetch_player_list() called");

    let alphabet = [
        'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r',
        's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
    ];
    let mut stream =
        futures::stream::iter((0..3).map(|_| alphabet.iter()).multi_cartesian_product()).fuse();

    let mut async_tasks = FuturesUnordered::new();
    let mut logger = ProgressLogger::new_with_target("fetch_player_list", 26 * 26 * 26);
    let mut player_list = vec![];
    loop {
        tokio::select! {
            Some(letters) = stream.next(), if async_tasks.len() < 100 => {
                let client = client.fork();
                async_tasks.push(async move {
                    let s: String = letters.iter().map(|c| *c).collect();
                    client.list_players(&s).await.map(|x| Some(x)).unwrap_or(None)
                });
            }
            Some(players) = async_tasks.next() => {
                //logger.increment(1);
                match players {
                    Some(mut v) => {
                        logger.increment(1);
                        player_list.append(&mut v);
                    }
                    None => {
                        println!("Something went wrong retrieving players!");
                    }
                }
            }
            else => break,
        }
    }

    // Save the player list
    let mut player_lookup: HashMap<String, u64> = HashMap::new();
    for player in player_list.iter() {
        player_lookup.insert(player.nickname.to_lowercase(), player.account_id);
    }
    player_lookup
}
