#![recursion_limit="256"]
#![feature(proc_macro_hygiene, decl_macro, future_readiness_fns)]
use serde_derive::{Deserialize, Serialize};
use futures::stream::StreamExt;
use futures::future::FutureExt;
use futures::stream::FuturesUnordered;
use std::io::{Read};
use tokio::fs::File;
use tokio::prelude::*;
use std::collections::HashMap;
//use histogram::Histogram;
use std::convert::TryInto;
use std::convert::Infallible;
use std::net::SocketAddr;
//use hyper::{Body, Request, Response, Server};
//use hyper::service::{make_service_fn, service_fn};
use std::sync::{Arc, Mutex};
use std::cell::Cell;
use rocket::State;
use rocket::http::RawStr;
use std::io::BufReader;
use itertools::*;
use stream_throttle::ThrottledStream;
use flate2::Compression;
use flate2::read::{GzEncoder, GzDecoder};
use std::time::Instant;
use std::time;

#[macro_use] extern crate rocket;

mod error;
mod wows_data;
mod scraper;
mod histogram;
mod progress_logger;
mod statistics;
mod database;

use error::Error;
use wows_data::*;
use scraper::WowsClient;
use crate::histogram::Histogram;
use crate::progress_logger::ProgressLogger;
use crate::statistics::*;
use crate::database::*;

/*struct ProgressLogger {
    tagline: String,
    last_report_time: Instant,
    item_count: usize,
    total: usize,
    target: Option<usize>,
}

impl ProgressLogger {
    pub fn new(tagline: &str) -> ProgressLogger {
        ProgressLogger {
            tagline: tagline.to_string(),
            last_report_time: Instant::now(),
            item_count: 0,
            total: 0,
            target: None,
        }
    }

    pub fn new_with_target(tagline: &str, target: usize) -> ProgressLogger {
        ProgressLogger {
            tagline: tagline.to_string(),
            last_report_time: Instant::now(),
            item_count: 0,
            total: 0,
            target: Some(target),
        }
    }

    pub fn increment(&mut self, count: usize) {
        self.item_count += count;
        self.total += count;
        let elapsed = self.last_report_time.elapsed().as_secs_f64();
        if elapsed > 60.0 {
            let rate = self.item_count as f64 / elapsed;
            match self.target {
                Some(target) => {
                    let remaining = if target < self.total { 0 } else { target - self.total };
                    println!("{}: {}/{} items. {} in {:.2}s = {:.2} items/sec ETA {:.0}s", self.tagline, self.total, target, self.item_count, elapsed, rate, remaining as f64 / rate);
                },
                None => {
                    println!("{}: {} items in {}s = {:.2} items/sec (total: {})", self.tagline, self.item_count, elapsed, self.item_count as f64 / elapsed, self.total);
                }
            }
            //self.total += self.item_count;
            self.last_report_time = Instant::now();
            self.item_count = 0;
        }
    }
}*/

/*#[derive(Debug, Serialize, Deserialize)]
struct AveragedBatteryStats {
    frags: f32,
    hits: f32,
    hitrate: f32,
    shots: f32,
}

#[derive(Debug, Serialize, Deserialize)]
struct AveragedShipStats {
    xp: f32,
    main_battery: AveragedBatteryStats,
    secondary_battery: AveragedBatteryStats,
    torpedoes: AveragedBatteryStats,
    ramming: AveragedBatteryStats,
    winrate: f32,
    win_survival_rate: f32,
    damage_dealt: f32,
    kills: f32,
    planes_killed: f32,
    points_captured: f32,
    spotted: f32,
    damage_on_spotting: f32,
}

impl AveragedBatteryStats {
    fn calculate(stats: &BatteryStats, num_battles: f32) -> AveragedBatteryStats {
        AveragedBatteryStats {
            frags: stats.frags as f32 / num_battles,
            hits: stats.hits as f32 / num_battles,
            hitrate: stats.hits as f32 / stats.shots as f32,
            shots: stats.shots as f32 / num_battles,
        }
    }
}

impl AveragedShipStats {
    fn calculate(stats: &DetailedStats) -> AveragedShipStats {
        let nbattles = (stats.wins + stats.losses) as f32;
        AveragedShipStats {
            xp: stats.xp as f32 / nbattles,
            main_battery: AveragedBatteryStats::calculate(&stats.main_battery, nbattles),
            secondary_battery: AveragedBatteryStats::calculate(&stats.second_battery, nbattles),
            torpedoes: AveragedBatteryStats::calculate(&stats.torpedoes, nbattles),
            ramming: AveragedBatteryStats::calculate(&stats.ramming, nbattles),
            winrate: stats.wins as f32 / nbattles,
            win_survival_rate: stats.survived_wins as f32 / stats.wins as f32,
            damage_dealt: stats.damage_dealt as f32 / nbattles,
            kills: stats.frags as f32 / nbattles,
            planes_killed: stats.planes_killed as f32 / nbattles,
            points_captured: stats.capture_points as f32 / nbattles,
            spotted: stats.ships_spotted as f32 / nbattles,
            damage_on_spotting: stats.damage_scouting as f32 / nbattles,
        }
    }
}

#[derive(Clone)]
struct BatteryHistogram {
    frags: Histogram,
    hits: Histogram,
    hitrate: Histogram,
    shots: Histogram,
}

#[derive(Clone)]
struct ShipStatsHistogram {
    xp: Histogram,
    main_battery: BatteryHistogram,
    secondary_battery: BatteryHistogram,
    torpedoes: BatteryHistogram,
    ramming: BatteryHistogram,
    winrate: Histogram,
    win_survival_rate: Histogram,
    damage_dealt: Histogram,
    kills: Histogram,
    planes_killed: Histogram,
    points_captured: Histogram,
    spotted: Histogram,
    damage_on_spotting: Histogram,
}

impl BatteryHistogram {
    fn new() -> BatteryHistogram {
        BatteryHistogram {
            frags: Histogram::new(20.0),
            hits: Histogram::new(1000.0),
            hitrate: Histogram::new(1.0),
            shots: Histogram::new(10_000.),
        }
    }

    fn increment(&mut self, value: &AveragedBatteryStats) -> Result<(), &'static str> {
        self.frags.increment(value.frags)?;
        self.hits.increment(value.hits)?;
        self.hitrate.increment(value.hitrate)?;
        self.shots.increment(value.shots)?;
        Ok(())
    }

    pub fn get_percentile(&self, value: &AveragedBatteryStats) -> Result<AveragedBatteryStats, &'static str> {
        Ok(AveragedBatteryStats {
            frags: self.frags.get_percentile(value.frags.into())? as f32,
            hits: self.hits.get_percentile(value.hits.into())? as f32,
            hitrate: self.hitrate.get_percentile(value.hitrate.into())? as f32,
            shots: self.shots.get_percentile(value.shots.into())? as f32,
        })
    }
}

impl ShipStatsHistogram {
    fn new() -> ShipStatsHistogram {
        ShipStatsHistogram {
            xp: Histogram::new(10_000.),
            main_battery: BatteryHistogram::new(),
            secondary_battery: BatteryHistogram::new(),
            torpedoes: BatteryHistogram::new(),
            ramming: BatteryHistogram::new(),
            winrate: Histogram::new(1.),
            win_survival_rate: Histogram::new(1.),
            damage_dealt: Histogram::new(1_000_000.),
            kills: Histogram::new(12.),
            planes_killed: Histogram::new(100.),
            points_captured: Histogram::new(10.),
            spotted: Histogram::new(100.),
            damage_on_spotting: Histogram::new(1_000_000.),
        }
    }

    fn increment(&mut self, value: &AveragedShipStats) -> Result<(), &'static str> {
        self.xp.increment(value.xp.into())?;
        self.main_battery.increment(&value.main_battery)?;
        self.secondary_battery.increment(&value.secondary_battery)?;
        self.torpedoes.increment(&value.torpedoes)?;
        self.ramming.increment(&value.ramming)?;
        self.winrate.increment(value.winrate.into())?;
        self.win_survival_rate.increment(value.win_survival_rate.into())?;
        self.damage_dealt.increment(value.damage_dealt.into())?;
        self.kills.increment(value.kills.into())?;
        self.planes_killed.increment(value.planes_killed.into())?;
        self.points_captured.increment(value.points_captured.into())?;
        self.spotted.increment(value.spotted.into())?;
        self.damage_on_spotting.increment(value.damage_on_spotting.into())?;
        Ok(())
    }

    pub fn get_percentile(&self, value: &AveragedShipStats) -> Result<AveragedShipStats, &'static str> {
        Ok(AveragedShipStats {
            xp: self.xp.get_percentile(value.xp.into())? as f32,
            main_battery: self.main_battery.get_percentile(&value.main_battery)?,
            secondary_battery: self.secondary_battery.get_percentile(&value.secondary_battery)?,
            torpedoes: self.torpedoes.get_percentile(&value.torpedoes)?,
            ramming: self.ramming.get_percentile(&value.ramming)?,
            winrate: self.winrate.get_percentile(value.winrate.into())? as f32,
            win_survival_rate: self.win_survival_rate.get_percentile(value.win_survival_rate.into())? as f32,
            damage_dealt: self.damage_dealt.get_percentile(value.damage_dealt.into())? as f32,
            kills: self.kills.get_percentile(value.kills.into())? as f32,
            planes_killed: self.planes_killed.get_percentile(value.planes_killed.into())? as f32,
            points_captured: self.points_captured.get_percentile(value.points_captured.into())? as f32,
            spotted: self.spotted.get_percentile(value.spotted.into())? as f32,
            damage_on_spotting: self.damage_on_spotting.get_percentile(value.damage_on_spotting.into())? as f32,
        })
    }
}*/

/*async fn hello_world(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
    Ok(Response::new("Hello, world!".into()))
}

async fn player(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    //
}*/

/*struct Database {
    //client: WowsClient,
    ships: HashMap<u64, ShipInfo>,
    stats: HashMap<u64, ShipStatsHistogram>,
    player_list: HashMap<String, u64>,
    player_data: Arc<sled::Db>,
}

impl Database {
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>, Error> {
        let mut gz = GzEncoder::new(data, Compression::fast());
        let mut body = vec!();
        let count = gz.read_to_end(&mut body)?;
        //f.write_all(&body).await.expect("Unable to fill cache file");
        Ok(body)
    }

    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>, Error> {
        let mut gz = GzDecoder::new(&data[..]);

        let mut body = vec!();
        gz.read_to_end(&mut body)?;
        Ok(body)
    }

    pub async fn new() -> Result<Database, Error> {
        let ships: HashMap<u64, ShipInfo> = {
            let mut f = File::open("ships.bin").await?;
            let mut v = vec!();
            f.read_to_end(&mut v).await?;
            bincode::deserialize(&v)?
        };
        let player_list: Vec<PlayerRecord> = {
            let mut encoded_players = vec!();
            let mut f = File::open("playerlist.bin").await?.read_to_end(&mut encoded_players).await?;
            bincode::deserialize(&encoded_players)?
        };
        let mut player_lookup = HashMap::new();
        for player in player_list.iter() {
            player_lookup.insert(player.nickname.to_lowercase(), player.account_id);
        }
        let ship_stats_hist = calculate_aggregate_stats(player_list.len()).await.unwrap();
        //let mut ship_stats_hist = HashMap::new();
        Ok(Database {
            ships: ships,
            player_list: player_lookup,
            stats: ship_stats_hist,
            player_data: Arc::new(sled::open("player_data.sled").expect("Unable to create sled database!")),
        })
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

            /*let (player_id, player) = data.iter().next().unwrap();
        let player = match player {
            Some(x) => x,
            None => {
                return Err(Error::CacheEntryNotFound);
            }
        };

        Ok(player)*/
    }

    pub fn update_detailed_stats(&self, player_id: u64, stats: &Vec<DetailedStatTypes>) -> Result<(), Error> {
        let player_id = format!("detailed_stats:{}", player_id);
        self.player_data.insert(&player_id, self.compress(serde_json::to_string(stats)?.as_bytes())?).expect(&format!("Error adding player_id='{}' to the cache", player_id));
        Ok(())
    }
}*/

#[get("/")]
fn index() -> &'static str {
    "Hello there, world!"
}

#[get("/player/<username>")]
fn player_stats(username: &RawStr, database: State<Arc<Mutex<Database>>>) -> String {
    let database = database.lock().unwrap();
    //let pid: u64 = pid.parse().unwrap();
    let username = username.to_lowercase();
    let account_id = match database.get_user(&username) {
        None => {
            return format!("Username '{}' was not found!", username);
        }
        Some(account_id) => {
            account_id
        }
    };

    /*let player_ships: HashMap<String, Option<Vec<DetailedStatTypes>>> =
        futures::executor::block_on(database.client.get_detailed_stats(*account_id)).unwrap();*/
    let player = database.get_detailed_stats(account_id).unwrap();

    /*let (player_id, player) = player_ships.iter().next().unwrap();
    let player = match player {
        Some(player) => {
            player
        }
        None => {
            return "Couldn't find player's stats!".to_string();
        }
    };*/

    let mut result = String::new();
    result.push_str(&format!("Hello {}! account_id={}\n", username, account_id));

    for ship_stats in player.iter() {
        let ship_id = ship_stats.ship_id;
        /*let (ship, stats) = match (database.ships.get(&ship_id), database.stats.get(&ship_id)) {
            (Some(a), Some(b)) => { (a, b) }
            _ => {
                result.push_str(&format!("\nHm, couldn't find this ship ID={}!\n", ship_id));
                continue;
            }
        };*/
        let (ship, stats) = match database.get_ship_stats(ship_id) {
            Some(a) => { a }
            _ => {
                result.push_str(&format!("\nHm, couldn't find this ship ID={}!\n", ship_id));
                continue;
            }
        };

        let num_battles = ship_stats.pvp.wins + ship_stats.pvp.losses;
        let ship_stats = AveragedShipStats::calculate(&ship_stats.pvp);
        let percentiles = stats.get_percentile(&ship_stats).unwrap();
        result.push_str(&format!("\nShip: Tier {} {} {} {} ({} battles played)\n", ship.tier, ship.nation, ship.ship_type, ship.name, num_battles));
        result.push_str(&format!(" - Damage: {:.0} (better than {:.1}% of players on this ship)\n", ship_stats.damage_dealt, percentiles.damage_dealt));
        result.push_str(&format!(" - Kills: {:.2} (better than {:.1}% of players on this ship)\n", ship_stats.kills, percentiles.kills));
        result.push_str(&format!(" - Main battery hit rate: {:.1}% (better than {:.1}% of players on this ship)\n", ship_stats.main_battery.hitrate * 100., percentiles.main_battery.hitrate));
        result.push_str(&format!(" - Main battery shells fired: {:.0} (better than {:.1}% of players on this ship)\n", ship_stats.main_battery.shots, percentiles.main_battery.shots));
        result.push_str(&format!(" - Main battery hits: {:.0} (better than {:.1}% of players on this ship)\n", ship_stats.main_battery.hits, percentiles.main_battery.hits));
        result.push_str(&format!(" - Win rate: {:.1}% (better than {:.1}% of players on this ship)\n", ship_stats.winrate * 100., percentiles.winrate));
        result.push_str(&format!(" - XP: {:.0} (better than {:.1}% of players on this ship)\n", ship_stats.xp, percentiles.xp));
    }
    result
    //"Hello!".to_string()
}

/*async fn database_update_loop(database: Arc<Mutex<Database>>) {
    // Keep a few things updated:
    // - The list of players
    // - Each of those player's detailed stats
    // - The list of boats
    println!("Starting DB update loop...");
    let client = WowsClient::new(...);
    loop {
        let start = Instant::now();

        update(&client, database.clone()).await;

        println!("Finished database update in {}s", start.elapsed().as_secs());
        let period = 7*3600*24; // 1 week
        if start.elapsed().as_secs() < period {
            std::thread::sleep(time::Duration::from_secs(period - start.elapsed().as_secs()));
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
    for (k,v) in new_player_list.iter() {
        if !player_list.contains_key(k) {
            num_new_players += 1;
            player_list.insert(k.to_string(), *v);
        }
    }

    println!("Found {} new players! Saving to file", num_new_players);
    let res: Vec<u8> = bincode::serialize(&player_list).unwrap();
    {
        let mut f = File::create("playerlist.bin.new").await.expect("Could not create file");
        f.write_all(&res).await.expect("Could not populate playerlist file");
    }
    std::fs::rename("playerlist.bin.new", "playerlist.bin").expect("Could not move playerlist file");

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

async fn calculate_aggregate_stats(nplayers: usize) -> Result<HashMap<u64, ShipStatsHistogram>, Error> {
    println!("calculate_aggregate_stats({} players) called", nplayers);
    let mut f = std::fs::File::open("detailed_stats.bin")?;
    let mut f = BufReader::new(f);
    //println!("Opened file");
    let mut ship_stats_hist = HashMap::new();
    let mut logger = ProgressLogger::new_with_target("calculate_aggregate_stats", nplayers);
    loop {
        let mut buffer = [0; 4];
        match f.read_exact(&mut buffer[..]) {
            Ok(_) => {}
            Err(e) => { /*println!("Read error {:?}", e);*/ break; }
        }
        let length = u32::from_le_bytes(buffer);
        //println!("Read {} bytes", length);
        let mut buffer = vec!();
        buffer.resize(length.try_into().unwrap(), 0);
        match f.read_exact(&mut buffer[..]) {
            Ok(_) => {},
            Err(e) => { println!("Read error {:?}", e); break; },
        }

        let stats: HashMap<String, Option<Vec<DetailedStatTypes>>> = match bincode::deserialize(&buffer) {
            Ok(x) => { x },
            Err(_) => { println!("Error bindecoding"); break; },
        };
        for (player_id,stats) in stats.iter() {
            //println!("Player: {}", player_id);
            //pb.inc(1);
            match stats {
                Some(stats) => {
                    for ship_stats in stats.iter() {
                        //println!("{}", ship_stats.ship_id);
                        if ship_stats.pvp.wins + ship_stats.pvp.losses < 10 {
                            continue;
                        }
                        if !ship_stats_hist.contains_key(&ship_stats.ship_id) {
                            ship_stats_hist.insert(ship_stats.ship_id, ShipStatsHistogram::new());
                        }
                        ship_stats_hist.get_mut(&ship_stats.ship_id).unwrap().increment(&AveragedShipStats::calculate(&ship_stats.pvp)).unwrap();
                    }
                }
                None => {}
            }
        }
    }
    Ok(ship_stats_hist)
}

/// Due to the size of this dataset, we write it out to a file
async fn fetch_player_data(client: &WowsClient, player_list: &HashMap<String, u64>, database: Arc<Mutex<Database>>) -> Result<(), Error> {
    println!("fetch_player_data({} players) called", player_list.len());
    let mut async_tasks = FuturesUnordered::new();
    let mut player_stream = futures::stream::iter(player_list.iter().map(|(name, account_id)| account_id)).fuse();
    let mut f = File::create("detailed_stats.bin.new").await?;
    let mut logger = ProgressLogger::new_with_target("fetch_player_data", player_list.len());
    loop {
        tokio::select! {
            account_id = player_stream.select_next_some().fuse(), if async_tasks.len() < 100 => {
                let client = client.fork();
                async_tasks.push(async move {
                    (
                        account_id,
                        client.get_detailed_stats(*account_id).await,//.map(|x| Some(x)).unwrap_or(None),
                    )
                });
            }
            stats = async_tasks.select_next_some().fuse() => {
                match stats {
                    (account_id, Ok(mut stats)) => {
                        // Stick these in a list or database or something
                        //pb.inc(1);
                        logger.increment(1);

                        let serialized = bincode::serialize(&stats)?;
                        let length: u32 = serialized.len().try_into().unwrap();
                        f.write_all(&length.to_le_bytes()).await?;
                        f.write_all(&serialized).await?;

                        {
                            let (player_id, player) = stats.iter().next().unwrap();
                            let player = match player {
                                Some(x) => {
                                    let mut database = database.lock().unwrap();
                                    database.update_detailed_stats(*account_id, x);
                                }
                                None => {
                                }
                            };
                        }

                        //player_count += v.len();
                        //println!("Found {} players! Count = {} in-flight futures={}", v.len(), player_count, async_tasks.len());
                        //player_list.append(&mut v);
                    }
                    (account_id, Err(e)) => {
                        println!("Something went wrong retrieving player {}! Error: {:?}", account_id, e);
                    }
                }
            }
            else => {},//break,
            //complete => break,
        }
    }
    std::fs::rename("detailed_stats.bin.new", "detailed_stats.bin").expect("Could not move detailed_stats file!");
    Ok(())
}

async fn fetch_player_list(client: &WowsClient) -> HashMap<String, u64> {
    println!("fetch_player_list() called");
    /*let client = {
        let database = database.lock().unwrap();
        database.client.fork()
    };*/

    // 1 item per 10 seconds: ~48 hours for the whole set
    /*let rate = stream_throttle::ThrottleRate::new(1, std::time::Duration::new(10, 0));
    let pool = stream_throttle::ThrottlePool::new(rate);
    let mut stream = futures::stream::iter((0..3).map(|_| {
        ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z'].iter()
    }).multi_cartesian_product()).throttle(pool).fuse();*/
    let mut stream = futures::stream::iter((0..3).map(|_| {
        ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z'].iter()
    }).multi_cartesian_product()).fuse();

    let mut async_tasks = FuturesUnordered::new();
    //let mut player_count = 0;
    let mut logger = ProgressLogger::new_with_target("fetch_player_list", 26*26*26);
    let mut player_list = vec!();
    loop {
        tokio::select! {
            letters = stream.select_next_some(), if async_tasks.len() < 100 => {
                let client = client.fork();
                async_tasks.push(async move {
                    let s: String = letters.iter().map(|c| *c).collect();
                    client.list_players(&s).await.map(|x| Some(x)).unwrap_or(None)
                });
            }
            players = async_tasks.select_next_some() => {
                match players {
                    Some(mut v) => {
                        logger.increment(1);
                        //player_count += v.len();
                        //println!("Found {} players! Count = {} in-flight futures={}", v.len(), player_count, async_tasks.len());
                        player_list.append(&mut v);
                    }
                    None => {
                        println!("Something went wrong retrieving players!");
                    }
                }
            }
            else => {},//break,
        }
    }

    // Save the player list
    let mut player_lookup = HashMap::new();
    for player in player_list.iter() {
        player_lookup.insert(player.nickname.to_lowercase(), player.account_id);
    }
    /*{
        let mut database = database.lock().unwrap();
        database.player_list = player_lookup;
    }*/
    player_lookup
}*/

#[tokio::main]
async fn main() -> Result<(), Error> {
    let ships: HashMap<u64, ShipInfo> = {
        let mut f = File::open("ships.bin").await?;
        let mut v = vec!();
        f.read_to_end(&mut v).await?;
        bincode::deserialize(&v)?
    };

    // Categorize every (T7-T9) ship and their speed
    /*let mut speed_map = HashMap::new();
    for (shipid,ship) in ships.iter() {
        if ship.tier >= 7 && ship.tier <= 9 {
            //println!("Tier {} {} {} {}: {}kts", ship.tier, ship.nation, ship.ship_type, ship.name, ship.default_profile.mobility.max_speed);
            println!("{},{},{},\"{}\",{}", ship.tier, ship.nation, ship.ship_type, ship.name, ship.default_profile.mobility.max_speed);
            if !speed_map.contains_key(&ship.nation) {
                speed_map.insert(ship.nation.clone(), HashMap::new());
            }
            let mut nmap = speed_map.get_mut(&ship.nation).unwrap();
            if !nmap.contains_key(&ship.ship_type) {
                nmap.insert(ship.ship_type.clone(), vec!());
            }
            nmap.get_mut(&ship.ship_type).unwrap().push(ship);
        }
    }
    for (nation,nation_ships) in speed_map.iter() {
        for (ship_type,ships) in nation_ships.iter() {
            let mut speeds: Vec<_> = ships.iter().map(|ship| { ship.default_profile.mobility.max_speed }).collect();
            speeds.sort_by(|a, b| { a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal) });
            let min_speed = speeds.iter().min_by(|a, b| { a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal) }).unwrap();
            let max_speed = speeds.iter().max_by(|a, b| { a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal) }).unwrap();
            if max_speed - min_speed > 6.0 {
                println!("{} {}: {}kts to {}kts, {}-{} without first/last", nation, ship_type, min_speed, max_speed, speeds[1], speeds[speeds.len()-2]);
            }
        }
    }

    return Ok(());*/

    //let client = WowsClient::new(...);
    /*let ships = client.enumerate_ships().await?;
    {
        println!("Dumping full ship list...");
        let res: Vec<u8> = bincode::serialize(&ships).unwrap();
        let mut f = File::create("ships.bin").await?;
        f.write_all(&res).await?;
    }*/

    /*let mut stream = futures::stream::iter((0..3).map(|_| {
        ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z'].iter()
    }).multi_cartesian_product()).fuse();

    let mut async_tasks = FuturesUnordered::new();
    let mut player_count = 0;
    let mut player_list = vec!();
    loop {
        select! {
            letters = stream.select_next_some() => {
                let client = client.fork();
                async_tasks.push(async move {
                    let s: String = letters.iter().map(|c| *c).collect();
                    client.list_players(&s).await.map(|x| Some(x)).unwrap_or(None)
                });
            }
            players = async_tasks.select_next_some() => {
                match players {
                    Some(mut v) => {
                        player_count += v.len();
                        println!("Found {} players! Count = {} in-flight futures={}", v.len(), player_count, async_tasks.len());
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
    {
        println!("Dumping full player list...");
        let res: Vec<u8> = bincode::serialize(&player_list).unwrap();
        let mut f = File::create("playerlist.bin").await?;
        f.write_all(&res).await?;
    }*/

    let player_list: Vec<PlayerRecord> = {
        let mut encoded_players = vec!();
        let mut f = File::open("playerlist.bin").await?.read_to_end(&mut encoded_players).await?;
        bincode::deserialize(&encoded_players)?
    };
    println!("Found {} players!", player_list.len());

    //let r = client.get_detailed_stats(1037107442).await?;
    //println!("{:#?}", r);

    /*let mut async_tasks = FuturesUnordered::new();

    let mut pb = indicatif::ProgressBar::new(player_list.len() as u64);
    pb.set_style(indicatif::ProgressStyle::default_bar()
                 .template("[{elapsed_precise}] {wide_bar} {pos:>7}/{len:7} ETA:{eta_precise}"));
    let mut player_stream = futures::stream::iter(player_list.iter()).fuse();
    //let mut f = File::create("detailed_stats.bin").await?;
    loop {
        tokio::select! {
            player = player_stream.select_next_some().fuse(), if async_tasks.len() < 100 => {
                let client = client.fork();
                async_tasks.push(async move {
                    (
                        player.account_id,
                        client.get_detailed_stats(player.account_id).await,//.map(|x| Some(x)).unwrap_or(None),
                    )
                });
            }
            stats = async_tasks.select_next_some().fuse() => {
                match stats {
                    (account_id, Ok(mut stats)) => {
                        // Stick these in a list or database or something
                        pb.inc(1);

    let serialized = bincode::serialize(&stats)?;
                        let length: u32 = serialized.len().try_into().unwrap();
                        f.write_all(&length.to_le_bytes()).await?;
                        f.write_all(&serialized).await?;

                        //player_count += v.len();
                        //println!("Found {} players! Count = {} in-flight futures={}", v.len(), player_count, async_tasks.len());
                        //player_list.append(&mut v);
                    }
                    (account_id, Err(e)) => {
                        println!("Something went wrong retrieving player {}! Error: {:?}", account_id, e);
                    }
                }
            }
            else => break,
            //complete => break,
        }
}*/

    /*let mut pb = indicatif::ProgressBar::new(1_400_000);
    pb.set_style(indicatif::ProgressStyle::default_bar()
                 .template("[{elapsed_precise}] {wide_bar} {pos:>7}/{len:7} ETA:{eta_precise}"));

    let mut f = std::fs::File::open("detailed_stats.bin")?;
    let mut f = BufReader::new(f);
    //println!("Opened file");
    let mut ship_stats_hist = HashMap::new();
    let database = Database::new().await.unwrap();
    loop {
        let mut buffer = [0; 4];
        match f.read_exact(&mut buffer[..]) {
            Ok(_) => {}
            Err(e) => { println!("Read error {:?}", e); break; }
        }
        let length = u32::from_le_bytes(buffer);
        //println!("Read {} bytes", length);
        let mut buffer = vec!();
        buffer.resize(length.try_into().unwrap(), 0);
        match f.read_exact(&mut buffer[..]) {
            Ok(_) => {},
            Err(e) => { println!("Read error {:?}", e); break; },
        }

        let stats: HashMap<String, Option<Vec<DetailedStatTypes>>> = match bincode::deserialize(&buffer) {
            Ok(x) => { x },
            Err(_) => { println!("Error bindecoding"); break; },
        };
        for (player_id,stats) in stats.iter() {
            //println!("Player: {}", player_id);
            pb.inc(1);
            match stats {
                Some(stats) => {
                    let account_id: u64 = player_id.parse().unwrap();
                    database.update_detailed_stats(account_id, stats).unwrap();

                    for ship_stats in stats.iter() {
                        //println!("{}", ship_stats.ship_id);
                        if ship_stats.pvp.wins + ship_stats.pvp.losses < 10 {
                            continue;
                        }
                        if !ship_stats_hist.contains_key(&ship_stats.ship_id) {
                            ship_stats_hist.insert(ship_stats.ship_id, ShipStatsHistogram::new());
                        }
                        ship_stats_hist.get_mut(&ship_stats.ship_id).unwrap().increment(&AveragedShipStats::calculate(&ship_stats.pvp)).unwrap();
                    }
                }
                None => {}
            }
        }
    }

    for (k,v) in ship_stats_hist.iter() {
        if !ships.contains_key(k) {
            println!("Could not find ship ID {}", k);
            continue;
        }
        println!("Ship: {} Damage: {}/{}/{}", ships.get(k).unwrap().name, v.damage_dealt.percentile(25.0).unwrap(), v.damage_dealt.percentile(50.0).unwrap(), v.damage_dealt.percentile(75.0).unwrap());
}*/

    let mut settings = config::Config::default();
    settings
        .merge(config::File::with_name("settings")).expect("Unable to find settings.toml file!")
        .merge(config::Environment::with_prefix("STATS")).unwrap();

    let settings: HashMap<String, String> = settings.try_into().unwrap();

    println!("{:?}", settings);

    let api_key = settings.get("api_key").expect("Could not find 'api_key' in settings");
    println!("{}", api_key);

    //return Ok(());

    let mut player_lookup = HashMap::new();
    for player in player_list.iter() {
        player_lookup.insert(player.nickname.to_lowercase(), player.account_id);
    }

    /*let database = Arc::new(Mutex::new(Database {
        client: client,
        ships: ships,
        stats: ship_stats_hist,
        player_list: player_lookup,
    }));*/

    let database = Arc::new(Mutex::new(Database::new().await.unwrap()));

    {
        let database = database.clone();
        std::thread::spawn(|| {
            rocket::ignite()
                .manage(database)
                .mount("/", routes![index, player_stats])
                .launch();
        });
        /*tokio::spawn(async {
            database_update_loop(database).await;
        });*/
    }

    println!("Starting database update thread");
    //futures::executor::block_on(database_update_loop(database));
    database_update_loop(api_key, database).await;
    /*loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }*/

    Ok(())
}
