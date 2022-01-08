#![recursion_limit = "256"]
#![feature(proc_macro_hygiene, decl_macro, future_readiness_fns)]
use futures::TryStreamExt;
use itertools::Itertools;
#[macro_use]
use log::*;
use mongodb::bson::doc;
use rocket::http::RawStr;
use rocket::State;
use rusoto_s3::S3;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use tera::{Context, Tera};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

//#[macro_use]
//extern crate rocket;

use rocket::{get, routes};

mod cheatsheet;
mod database;
mod error;
mod gameparams;
mod histogram;
mod progress_logger;
mod scraper;
mod statistics;
mod wows_data;

use crate::cheatsheet::CheatsheetDb;
use crate::database::*;
use crate::gameparams::GameParams;
use crate::statistics::*;
use error::Error;
use wows_data::*;

#[get("/")]
fn index() -> &'static str {
    "Hello there! Go ahead and go to the URL /warshipstats/player/<your username> to see your stats."
}

/*
#[get("/cheatsheet/<tier>")]
fn cheatsheet(tier: u16, database: State<CheatsheetDb>) -> rocket::response::content::Html<String> {
    let mut tera = Tera::new("templates/ *").unwrap();
    tera.add_raw_template(
        "cheatsheet.html",
        std::include_str!("../templates/cheatsheet.html"),
    )
    .unwrap();
    tera.register_filter(
        "shortclass",
        |class: &tera::Value, _: &HashMap<String, tera::Value>| {
            let class: crate::cheatsheet::ShipClass =
                serde_json::value::from_value(class.clone()).unwrap();
            Ok(serde_json::value::to_value(class.short()).unwrap())
        },
    );
    tera.register_tester("none", |value: Option<&tera::Value>, _: &[tera::Value]| {
        Ok(value.unwrap().is_null())
    });
    tera.register_filter(
        "unwrap_float",
        |value: &tera::Value, _: &HashMap<String, tera::Value>| {
            let value: Option<f32> = serde_json::value::from_value(value.clone()).unwrap();
            let value = value.unwrap();
            Ok(serde_json::value::to_value(value).unwrap())
        },
    );

    //let mut result = String::new();
    let mut ships = vec![];
    for (id, ship) in database.ships.iter() {
        if ship.min_tier <= tier && ship.max_tier >= tier {
            ships.push(ship);
        }
    }
    ships.sort_by_key(|ship| (ship.class, &ship.name));

    let mut context = HashMap::new();
    context.insert("ships", &ships);
    rocket::response::content::Html(
        tera.render(
            "cheatsheet.html",
            &Context::from_serialize(&context).unwrap(),
        )
        .unwrap(),
    )
}
*/

#[get("/player/<username>")]
async fn player_stats(
    username: &str,
    database: &State<mongodb::Database>,
    histograms: &State<Arc<Mutex<StatsHistogram>>>,
) -> String {
    // Get the player's ID
    let username = username.to_lowercase();
    let collection = database.collection::<PlayerRecord>("playerids");
    let filter = doc! { "nickname": username.clone() };
    let record = collection.find_one(filter, None).await.unwrap().unwrap();

    // Get the player's stats
    let collection = database.collection::<DetailedStatRecord>("playerstats");
    let filter = doc! { "account_id": record.account_id as i64 };
    let mut cursor = collection.find(filter, None).await.unwrap();

    //format!("{} {:?}", record.account_id, stats)

    /*let database = database.lock().unwrap();
    //let pid: u64 = pid.parse().unwrap();
    let username = username.to_lowercase();
    let account_id = match database.get_user(&username) {
        None => {
            return format!("Username '{}' was not found!", username);
        }
        Some(account_id) => account_id,
    };

    let player = database.get_detailed_stats(account_id).unwrap();*/

    let mut result = String::new();
    result.push_str(&format!(
        "Hello {}! account_id={}\n",
        username, record.account_id
    ));

    //for ship_stats in player.iter() {
    while let Some(ship_stats) = cursor.try_next().await.unwrap() {
        let ship_id = ship_stats.ship_id;

        let histograms = histograms.lock().unwrap();
        let percentiles = histograms.get_percentiles(ship_id, &ship_stats.pvp);

        result.push_str(&format!("{:?}\n", percentiles));

        /*let (ship, stats) = match database.get_ship_stats(ship_id) {
            Some(a) => a,
            _ => {
                result.push_str(&format!("\nHm, couldn't find this ship ID={}!\n", ship_id));
                continue;
            }
        };

        let num_battles = ship_stats.pvp.wins + ship_stats.pvp.losses;
        let ship_stats = AveragedShipStats::calculate(&ship_stats.pvp);
        let percentiles = stats.get_percentile(&ship_stats).unwrap();
        result.push_str(&format!(
            "\nShip: Tier {} {} {} {} ({} battles played)\n",
            ship.tier, ship.nation, ship.ship_type, ship.name, num_battles
        ));
        result.push_str(&format!(
            " - Damage: {:.0} (better than {:.1}% of players on this ship)\n",
            ship_stats.damage_dealt, percentiles.damage_dealt
        ));
        result.push_str(&format!(
            " - Kills: {:.2} (better than {:.1}% of players on this ship)\n",
            ship_stats.kills, percentiles.kills
        ));
        result.push_str(&format!(
            " - Main battery hit rate: {:.1}% (better than {:.1}% of players on this ship)\n",
            ship_stats.main_battery.hitrate * 100.,
            percentiles.main_battery.hitrate
        ));
        result.push_str(&format!(
            " - Main battery shells fired: {:.0} (better than {:.1}% of players on this ship)\n",
            ship_stats.main_battery.shots, percentiles.main_battery.shots
        ));
        result.push_str(&format!(
            " - Main battery hits: {:.0} (better than {:.1}% of players on this ship)\n",
            ship_stats.main_battery.hits, percentiles.main_battery.hits
        ));
        result.push_str(&format!(
            " - Win rate: {:.1}% (better than {:.1}% of players on this ship)\n",
            ship_stats.winrate * 100.,
            percentiles.winrate
        ));
        result.push_str(&format!(
            " - XP: {:.0} (better than {:.1}% of players on this ship)\n",
            ship_stats.xp, percentiles.xp
        ));*/
    }
    result
}

struct Config {
    api_key: String,
    request_period: u64,
}

impl Config {
    fn from_map(settings: HashMap<String, String>) -> Config {
        let api_key = settings
            .get("api_key")
            .expect("Could not find 'api_key' in settings")
            .to_string();
        let request_rate: f64 = settings
            .get("api_request_rate")
            .expect("Could not find 'api_request_rate' in settings")
            .parse()
            .expect("Could not parse api_request_rate as a float");
        let request_period: u64 = (1_000_000_000.0 / request_rate) as u64;
        Config {
            api_key,
            request_period,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Config;
    use std::collections::HashMap;

    #[test]
    fn config_parser_works() {
        let mut settings = HashMap::new();
        settings.insert("api_key".to_string(), "asdf".to_string());
        settings.insert("api_request_rate".to_string(), "20".to_string());

        let cfg = Config::from_map(settings);
        assert_eq!(cfg.api_key, "asdf");
        assert_eq!(cfg.request_period, 50_000_000);
    }
}

async fn try_load<T: DeserializeOwned>(filename: &str) -> Result<T, Error> {
    let mut data = vec![];
    File::open(filename).await?.read_to_end(&mut data).await?;
    // Try bincode first...
    bincode::deserialize(&data).or_else(|_| {
        std::str::from_utf8(&data)
            .map_err(|e| Error::from(e))
            .and_then(|s| serde_json::from_str(s).map_err(|e| Error::from(e)))
    })
}

async fn load_or_do<
    T: DeserializeOwned + Serialize,
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<T, Error>>,
>(
    filename: &str,
    json: bool,
    cb: F,
) -> Result<T, Error> {
    match try_load(filename).await {
        Ok(x) => Ok(x),
        Err(_) => {
            let value = cb().await?;
            let serialized = if json {
                serde_json::to_string_pretty(&value)?.into_bytes()
            } else {
                bincode::serialize(&value)?
            };
            File::create(filename).await?.write_all(&serialized).await?;
            Ok(value)
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    console_subscriber::init();

    let mut settings = config::Config::default();
    settings
        .merge(config::File::with_name("settings"))
        .expect("Unable to find settings.toml file!")
        .merge(config::Environment::with_prefix("STATS"))
        .unwrap();

    let settings: HashMap<String, String> = settings.try_into().unwrap();
    let cfg = Config::from_map(settings);

    let storage_client = mongodb::Client::with_options(
        mongodb::options::ClientOptions::parse("mongodb://localhost:27017")
            .await
            .unwrap(),
    )
    .unwrap();
    let db = storage_client.database("wows_player_stats");

    let collection = db.collection::<database::DetailedStatRecord>("playerstats");
    let stats_count = collection.count_documents(None, None).await.unwrap();
    info!("DB has {} player+ship entries already", stats_count);
    if stats_count == 0 {
        let mut index = bson::Document::new();
        index.insert("account_id", 1u32);
        collection
            .create_index(mongodb::IndexModel::builder().keys(index).build(), None)
            .await;
    }

    let collection = db.collection::<PlayerRecord>("playerids");
    if collection.count_documents(None, None).await.unwrap() == 0 {
        let mut index = bson::Document::new();
        index.insert("nickname", 1u32);
        collection
            .create_index(mongodb::IndexModel::builder().keys(index).build(), None)
            .await;
    }

    let mut histograms = StatsHistogram::new();
    histograms.set_database_size(stats_count);

    // Prime the histograms with all the current statistics
    info!("Priming histogram with existing DB entries");
    let collection = db.collection::<database::DetailedStatRecord>("playerstats");
    let mut cursor = collection.find(None, None).await.unwrap();
    while let Some(statrecord) = cursor.try_next().await.unwrap() {
        histograms.increment(statrecord.ship_id, &statrecord.pvp);
    }
    let histograms = Arc::new(Mutex::new(histograms));

    info!("Starting app");
    let client = crate::scraper::WowsClient::new(&cfg.api_key, cfg.request_period);

    {
        let db = db.clone();
        let histograms = histograms.clone();
        tokio::spawn(async move {
            database::poller(&client, db, histograms).await;
        });
    }

    let database = db.clone();
    rocket::build()
        .manage(database)
        .manage(histograms)
        .mount(
            "/warshipstats",
            routes![index, player_stats /*, cheatsheet*/],
        )
        .launch()
        .await;

    // Generate cheatsheet
    /*let gameparams: GameParams = {
        //let data = std::include_bytes!("../GameParams.json");
        //GameParams::load(&GAME_PARAMS)?
        let mut data = vec![];
        File::open("GameParams.json")
            .await
            .expect("Could not open required GameParams.json file")
            .read_to_end(&mut data)
            .await?;
        GameParams::load(&data)?
        //let mut data = std::include_bytes!("../GameParams.data");
        //GameParams::load_raw(&data[..]).unwrap()
    };

    println!("Loading ships...");
    let ships = load_or_do("ships.dat", true, || async {
        client.enumerate_ships().await
    })
    .await?;

    println!("Found {} ships", ships.len());

    // Get all the detailed module infos
    let modules = load_or_do("modules.dat", true, || async {
        let mut moduleids = HashSet::new();
        for (_, ship) in ships.iter() {
            for (_, module) in ship.modules_tree.iter() {
                moduleids.insert(module.module_id);
            }
        }

        println!("Downloading {} modules...", moduleids.len());

        let mut modules = HashMap::new();
        for moduleids in &moduleids.iter().chunks(100) {
            let chunk: Vec<_> = moduleids.map(|x| *x).collect();
            for (id, module) in client.get_module_info(chunk.as_slice()).await?.iter() {
                modules.insert(*id, module.clone());
            }
        }
        Ok(modules)
    })
    .await?;

    println!("Creating cheatsheet DB...");
    let cheatsheetdb = load_or_do("cheatsheet.dat", true, || async {
        Ok(crate::cheatsheet::CheatsheetDb::from(
            &ships,
            &gameparams,
            &modules,
        ))
    })
    .await?;

    /*rocket::ignite()
    .manage(cheatsheetdb)
    .mount("/warshipstats", routes![index, cheatsheet])
    .launch();*/

    println!("Enumerating playerlist...");
    let player_list = load_or_do("playerlist.dat", true, || async {
        /*let chars = vec![
            'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q',
            'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
        ];
        let mut players = vec![];
        for chars in (0u8..3).map(|_| chars.clone()).multi_cartesian_product() {
            //println!("{:?}", chars);
            let chars: Vec<_> = chars.iter().map(|c| format!("{}", c)).collect();
            let chars: String = (&chars[..]).join("");
            //println!("{}", chars);
            players.append(&mut client.list_players(&chars).await?);
        }
        Ok(players)*/
        let (_, players) = crate::database::fetch_player_list(&client).await;
        Ok(players)
    })
    .await?;

    /*let player_list: Vec<PlayerRecord> = {
        let mut encoded_players = vec![];
        File::open("playerlist.bin")
            .await?
            .read_to_end(&mut encoded_players)
            .await?;
        bincode::deserialize(&encoded_players)?
    };*/
    println!("Found {} players!", player_list.len());

    let mut player_lookup = HashMap::new();
    for player in player_list.iter() {
        player_lookup.insert(player.nickname.to_lowercase(), player.account_id);
    }

    println!("Downloading detailed stats");

    match File::open("detailed_stats.bin").await {
        Ok(_) => {
            // Already created, skip
        }
        Err(_) => {
            crate::database::init_player_data(&client, &player_lookup)
                .await
                .unwrap();
        }
    }

    println!("Creating player stats DB...");
    let database = Arc::new(Mutex::new(Database::new(ships, player_list).await.unwrap()));

    {
        let database = database.clone();
        std::thread::spawn(|| {
            rocket::ignite()
                .manage(database)
                .mount("/warshipstats", routes![index, player_stats, cheatsheet])
                .launch();
        });
    }

    println!("Starting database update thread");
    database_update_loop(&cfg.api_key, cfg.request_period, database).await;*/

    Ok(())
}
