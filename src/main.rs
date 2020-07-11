#![recursion_limit = "256"]
#![feature(proc_macro_hygiene, decl_macro, future_readiness_fns)]
use rocket::http::RawStr;
use rocket::State;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::fs::File;
use tokio::prelude::*;

#[macro_use]
extern crate rocket;

mod database;
mod error;
mod histogram;
mod progress_logger;
mod scraper;
mod statistics;
mod wows_data;

use crate::database::*;
use crate::statistics::*;
use error::Error;
use wows_data::*;

#[get("/")]
fn index() -> &'static str {
    "Hello there! Go ahead and go to the URL /warshipstats/player/<your username> to see your stats."
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
        Some(account_id) => account_id,
    };

    let player = database.get_detailed_stats(account_id).unwrap();

    let mut result = String::new();
    result.push_str(&format!("Hello {}! account_id={}\n", username, account_id));

    for ship_stats in player.iter() {
        let ship_id = ship_stats.ship_id;
        let (ship, stats) = match database.get_ship_stats(ship_id) {
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
        ));
    }
    result
}

struct Config {
    api_key: String,
    request_period: u64,
}

impl Config {
    fn from_map(settings: HashMap<String, String>) -> Config {
        let api_key = settings.get("api_key").expect("Could not find 'api_key' in settings").to_string();
        let request_rate: f64 = settings.get("api_request_rate").expect("Could not find 'api_request_rate' in settings").parse().expect("Could not parse api_request_rate as a float");
        let request_period: u64 = (1_000_000_000.0 / request_rate) as u64;
        Config {
            api_key,
            request_period,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use super::Config;

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
    let mut settings = config::Config::default();
    settings
        .merge(config::File::with_name("settings"))
        .expect("Unable to find settings.toml file!")
        .merge(config::Environment::with_prefix("STATS"))
        .unwrap();

    let settings: HashMap<String, String> = settings.try_into().unwrap();
    let cfg = Config::from_map(settings);

    let player_list: Vec<PlayerRecord> = {
        let mut encoded_players = vec![];
        File::open("playerlist.bin")
            .await?
            .read_to_end(&mut encoded_players)
            .await?;
        bincode::deserialize(&encoded_players)?
    };
    println!("Found {} players!", player_list.len());

    let mut player_lookup = HashMap::new();
    for player in player_list.iter() {
        player_lookup.insert(player.nickname.to_lowercase(), player.account_id);
    }

    let database = Arc::new(Mutex::new(Database::new().await.unwrap()));

    {
        let database = database.clone();
        std::thread::spawn(|| {
            rocket::ignite()
                .manage(database)
                .mount("/warshipstats", routes![index, player_stats])
                .launch();
        });
    }

    println!("Starting database update thread");
    database_update_loop(&cfg.api_key, cfg.request_period, database).await;

    Ok(())
}
