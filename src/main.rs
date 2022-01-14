#![recursion_limit = "256"]
#![feature(proc_macro_hygiene, decl_macro)]
use futures::TryStreamExt;
use mongodb::bson::doc;
use rocket::State;
use rocket::{get, routes};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tera::{Context, Tera};
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tracing::*;
use tracing_subscriber::prelude::*;

mod cheatsheet;
mod database;
mod error;
mod gameparams;
mod histogram;
mod progress_logger;
mod scraper;
mod ships;
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

#[get("/cheatsheet/<tier>")]
fn render_cheatsheet(
    tier: u16,
    database: &State<CheatsheetDb>,
) -> rocket::response::content::Html<String> {
    let mut tera = Tera::new("templates/*").unwrap();
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

    let mut ships = vec![];
    for id in database.enumerate_ships().iter() {
        let ship = database.get_ship(*id).unwrap();
        if ship.min_tier <= tier && ship.max_tier >= tier {
            ships.push(ship);
        }
    }
    ships.sort_by_key(|ship| (ship.class, ship.name.clone()));
    let ships: Vec<tera::Value> = ships
        .iter()
        .map(|x| serde_json::value::to_value(x).unwrap())
        .collect();

    let mut context: HashMap<&'static str, tera::Value> = HashMap::new();
    context.insert("tier", tier.into());
    context.insert("ships", ships.into());
    rocket::response::content::Html(
        tera.render(
            "cheatsheet.html",
            &Context::from_serialize(&context).unwrap(),
        )
        .unwrap(),
    )
}

async fn build_playerstats_context(
    username: &str,
    database: &mongodb::Database,
    histograms: &Arc<Mutex<StatsHistogram>>,
    shipdb: &crate::ships::ShipDb,
) -> HashMap<String, tera::Value> {
    // Get the player's ID
    let username = username.to_lowercase();
    let collection = database.collection::<PlayerRecord>("playerids");
    let filter = doc! { "nickname": username.clone() };
    let record = match collection.find_one(filter, None).await.unwrap() {
        Some(x) => x,
        None => {
            error!("Could not find username '{}'", username);
            let mut context: HashMap<String, tera::Value> = HashMap::new();
            context.insert(
                "error".to_owned(),
                format!("Could not find username '{}'", username).into(),
            );
            return context;
        }
    };

    // Get the player's stats
    let collection = database.collection::<DetailedStatRecord>("playerstats");
    let filter = doc! { "account_id": record.account_id as i64 };
    let mut cursor = collection.find(filter, None).await.unwrap();

    let mut context: HashMap<String, tera::Value> = HashMap::new();
    context.insert("error".to_owned(), (false).into());

    let mut ships: Vec<tera::Value> = vec![];
    while let Some(ship_stats) = cursor.try_next().await.unwrap() {
        let ship_id = ship_stats.ship_id;

        // How old the data is
        let data_age = chrono::Utc::now().signed_duration_since(ship_stats.retrieved);
        let age_formatter = timeago::Formatter::new();
        let data_age = age_formatter.convert(data_age.to_std().unwrap());
        context.insert("data_age".to_owned(), format!("{}", data_age).into());

        // Collect some meta information about the ship itself
        let mut ship: tera::Map<String, tera::Value> = tera::Map::new();

        if let Some(ship_info) = shipdb.get_ship_info(ship_id) {
            ship.insert("known".to_owned(), (true).into());
            ship.insert("tier".to_owned(), ship_info.tier.into());
            ship.insert("nation".to_owned(), ship_info.nation.into());
            ship.insert("ship_type".to_owned(), ship_info.ship_type.into());
            ship.insert("name".to_owned(), ship_info.name.into());
        } else {
            ship.insert("known".to_owned(), (false).into());
        }

        ship.insert("num_battles".to_owned(), ship_stats.battles.into());
        ship.insert("shipid".to_owned(), ship_id.into());

        // Collect the statistics about the player's performance on the ship
        let histograms = histograms.lock().unwrap();
        let percentiles = histograms.get_percentiles(ship_id, &ship_stats.pvp);

        let percentiles: tera::Map<String, tera::Value> = percentiles
            .iter()
            .map(|(k, v)| (k.to_owned(), (*v).into()))
            .collect();
        ship.insert("percentiles".to_owned(), percentiles.into());
        let ship_stats: tera::Map<String, tera::Value> = ship_stats
            .pvp
            .into_map()
            .iter()
            .map(|(k, v)| (k.to_owned(), (*v).into()))
            .collect();
        ship.insert("stats".to_owned(), ship_stats.into());

        ships.push(ship.into());
    }

    context.insert("ships".to_owned(), ships.into());
    context.insert("username".to_owned(), username.into());
    context
}

#[get("/ships")]
async fn ship_data(shipdb: &State<crate::ships::ShipDb>) -> String {
    let ships = shipdb.get_all_info();
    serde_json::to_string(&ships).unwrap()
}

#[get("/player-raw/<username>")]
async fn player_stats_raw(
    username: &str,
    database: &State<mongodb::Database>,
    histograms: &State<Arc<Mutex<StatsHistogram>>>,
    ships: &State<crate::ships::ShipDb>,
) -> String {
    let context = build_playerstats_context(username, database, histograms, ships).await;

    serde_json::to_string(&context).unwrap()
}

#[get("/player/<username>")]
async fn player_stats(
    username: &str,
    database: &State<mongodb::Database>,
    histograms: &State<Arc<Mutex<StatsHistogram>>>,
    ships: &State<crate::ships::ShipDb>,
) -> String {
    let context = build_playerstats_context(username, database, histograms, ships).await;

    let mut tera = Tera::new("templates/*").unwrap();
    tera.add_raw_template(
        "playerstats.txt",
        &std::fs::read_to_string("./templates/playerstats.txt")
            .unwrap_or(std::include_str!("../templates/playerstats.txt").to_string()),
    )
    .unwrap();

    tera.register_tester("none", |value: Option<&tera::Value>, _: &[tera::Value]| {
        Ok(value.unwrap().is_null())
    });
    tera.register_filter(
        "unwrap_float",
        |value: &tera::Value, _: &HashMap<String, tera::Value>| {
            let value: Option<f32> = serde_json::value::from_value(value.clone()).unwrap();
            let value = value.unwrap_or(0.0);
            Ok(serde_json::value::to_value(value).unwrap())
        },
    );
    tera.register_filter(
        "mult100",
        |value: &tera::Value, _: &HashMap<String, tera::Value>| {
            let value: f32 = serde_json::value::from_value(value.clone()).unwrap();
            Ok(serde_json::value::to_value(value * 100.0).unwrap())
        },
    );

    tera.render(
        "playerstats.txt",
        &Context::from_serialize(&context).unwrap(),
    )
    .unwrap()
}

struct Config {
    disable_scraper: bool,
    api_key: String,
    request_period: u64,
    mongo_url: String,
}

impl Config {
    fn from_map(settings: HashMap<String, String>) -> Config {
        let disable_scraper = match settings.get("disable_scraper") {
            Some(x) => x.parse().unwrap(),
            None => false,
        };
        let api_key = settings
            .get("api_key")
            .expect("Could not find 'api_key' in settings")
            .to_string();
        let request_rate: f64 = settings
            .get("api_request_rate")
            .expect("Could not find 'api_request_rate' in settings")
            .parse()
            .expect("Could not parse api_request_rate as a float");
        let mongo_url = settings
            .get("mongo")
            .expect("Could not find 'mongo' in settings")
            .to_string();
        let request_period: u64 = (1_000_000_000.0 / request_rate) as u64;
        Config {
            disable_scraper,
            api_key,
            request_period,
            mongo_url,
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

#[tokio::main]
async fn main() -> Result<(), Error> {
    let filter = tracing_subscriber::filter::Targets::new()
        .with_default(tracing::Level::WARN)
        .with_target("rocket", tracing::Level::DEBUG)
        .with_target("wows_player_stats", tracing::Level::TRACE)
        .with_target("wows_player_stats::histogram", tracing::Level::INFO);

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(filter)
        .init();

    //console_subscriber::init();

    let mut settings = config::Config::default();
    settings
        .merge(config::File::with_name("settings"))
        .expect("Unable to find settings.toml file!")
        .merge(config::Environment::with_prefix("STATS"))
        .unwrap();

    let settings: HashMap<String, String> = settings.try_into().unwrap();
    let cfg = Config::from_map(settings);

    let storage_client = mongodb::Client::with_options(
        mongodb::options::ClientOptions::parse(cfg.mongo_url)
            .await
            .unwrap(),
    )
    .unwrap();
    let db = storage_client.database("wows_player_stats");

    info!("Connected to DB. Counting entries...");
    let collection = db.collection::<database::DetailedStatRecord>("playerstats");
    let stats_count = collection.estimated_document_count(None).await.unwrap();
    info!("DB has {} player+ship entries already", stats_count);
    if stats_count == 0 {
        let index = doc! { "account_id": 1 };
        collection
            .create_index(mongodb::IndexModel::builder().keys(index).build(), None)
            .await
            .expect("Could not create index on playerstats collection");
    }

    let collection = db.collection::<PlayerRecord>("playerids");
    if collection.estimated_document_count(None).await.unwrap() == 0 {
        let index = doc! { "nickname": 1 };
        collection
            .create_index(mongodb::IndexModel::builder().keys(index).build(), None)
            .await
            .expect("Could not create index on playerids collection");
    }

    let histograms = Arc::new(Mutex::new(StatsHistogram::new()));

    {
        let db = db.clone();
        let histograms = histograms.clone();
        tokio::spawn(async move {
            // Prime the histograms with all the current statistics
            info!("Priming histogram with existing DB entries");
            let collection = db.collection::<database::DetailedStatRecord>("playerstats");
            let mut cursor = collection.find(None, None).await.unwrap();
            let mut pl = crate::progress_logger::ProgressLogger::new_with_target(
                "histogram_prime",
                stats_count as usize,
            );
            while let Some(statrecord) = cursor.try_next().await.unwrap() {
                let mut histograms = histograms.lock().unwrap();
                histograms.increment(statrecord.ship_id, &statrecord.pvp);
                pl.increment(1);
            }
            info!("Finished priming histograms");
        });
    }

    let ships = crate::ships::ShipDb::new();

    info!("Starting app");
    let client = crate::scraper::WowsClient::new(&cfg.api_key, cfg.request_period);

    // Load the cheatsheet
    let cheatsheetdb = {
        let gameparams: GameParams = {
            let mut data = vec![];
            File::open("GameParams.json")
                .await
                .expect("Could not open required GameParams.json file")
                .read_to_end(&mut data)
                .await?;
            GameParams::load(&data)?
        };
        let ships = ships.clone();
        CheatsheetDb::from(ships, gameparams)
    };

    // Scrape the WoWS API, and keep the histograms updated
    if !cfg.disable_scraper {
        let db = db.clone();
        let histograms = histograms.clone();
        let client = client.fork();
        tokio::spawn(async move {
            database::poller(&client, db, histograms).await;
        });
    }

    // Periodically (every hour) update the histograms with how big the database is
    {
        let db = db.clone();
        let histograms = histograms.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(3600 * 1000)).await;

                let collection = db.collection::<database::DetailedStatRecord>("playerstats");
                let stats_count = collection.count_documents(None, None).await.unwrap();
                info!("Database now contains {} entries", stats_count);
                let mut histograms = histograms.lock().unwrap();
                histograms.set_database_size(stats_count);
            }
        });
    }

    // Keep the ships database up-to-date
    {
        let ships = ships.clone();
        let client = client.fork();
        tokio::spawn(async move {
            ships.update_loop(client).await;
        });
    }

    // Run the web
    let database = db.clone();
    rocket::build()
        .manage(database)
        .manage(histograms)
        .manage(ships)
        .manage(cheatsheetdb)
        .mount(
            "/warshipstats",
            routes![
                index,
                player_stats,
                player_stats_raw,
                ship_data,
                render_cheatsheet
            ],
        )
        .launch()
        .await
        .expect("Issue running webserver");

    Ok(())
}
