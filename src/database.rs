use itertools::*;
use mongodb::bson::doc;
use serde_derive::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tracing::*;

use crate::scraper::WowsClient;
use crate::statistics::*;
use crate::wows_data::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DetailedStatRecord {
    pub pvp: DetailedStats,
    pub account_id: u64,
    pub ship_id: u64,
    pub battles: u64,
    pub retrieved: chrono::DateTime<chrono::Utc>,
}

pub async fn poller(
    client: &WowsClient,
    database: mongodb::Database,
    histograms: Arc<Mutex<StatsHistogram>>,
) {
    let (alphabet_sender, alphabet_receiver) = async_channel::bounded(256);

    let x = tokio::spawn(async move {
        let alphabet = [
            'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q',
            'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
        ];
        loop {
            let mut cnt = 0;
            for prefix in (0..3).map(|_| alphabet.iter()).multi_cartesian_product() {
                let prefix: String = prefix.iter().map(|c| *c).collect();
                if cnt > 1742 {
                    alphabet_sender.send(prefix).await;
                }
                cnt += 1;
            }
        }
    });

    // Have some workers to get the players for each prefix
    let (player_sender, player_receiver) = async_channel::bounded(1024);
    for _ in 0..10 {
        let client = client.fork();
        let player_sender = player_sender.clone();
        let alphabet_receiver = alphabet_receiver.clone();
        let database = database.clone();
        tokio::spawn(async move {
            while let Ok(prefix) = alphabet_receiver.recv().await {
                match client.list_players(&prefix).await {
                    Ok(players) => {
                        let players: Vec<PlayerRecord> = players
                            .iter()
                            .map(|player| PlayerRecord {
                                nickname: player.nickname.to_lowercase(),
                                account_id: player.account_id,
                            })
                            .collect();

                        if players.len() > 0 {
                            let collection = database.collection::<PlayerRecord>("playerids");
                            collection.insert_many(players.clone(), None).await.unwrap();

                            player_sender.send(players).await;
                        } else {
                            debug!("No players for prefix {}", prefix);
                        }
                    }
                    Err(e) => {
                        eprintln!(
                            "Got an error listing players for prefix {}: {:?}",
                            prefix, e
                        );
                    }
                }
            }
        });
    }

    // Have some workers to get detailed stats for the players
    for _ in 0..10 {
        let player_receiver = player_receiver.clone();
        let client = client.fork();
        let database = database.clone();
        let histograms = histograms.clone();
        tokio::spawn(async move {
            while let Ok(players) = player_receiver.recv().await {
                //println!("Got {} players!", players.len());
                for player in players.iter() {
                    match client.get_detailed_stats(player.account_id).await {
                        Ok(stats) => {
                            //println!("Got stats for player {}", player.nickname);

                            match stats.iter().next() {
                                Some((_player_id, stats)) => {
                                    match stats {
                                        Some(stats) => {
                                            let stats: Vec<DetailedStatRecord> = stats
                                                .iter()
                                                .map(|stat| DetailedStatRecord {
                                                    pvp: stat.pvp.clone(),
                                                    account_id: stat.account_id,
                                                    ship_id: stat.ship_id,
                                                    battles: stat.battles,
                                                    retrieved: chrono::Utc::now(),
                                                })
                                                .collect();

                                            // Update the histograms
                                            stats.iter().for_each(|stat| {
                                                let mut histograms = histograms.lock().unwrap();
                                                histograms.increment(stat.ship_id, &stat.pvp);
                                            });

                                            let collection = database
                                                .collection::<DetailedStatRecord>("playerstats");

                                            // TODO: This is a race condition, if a query for this account comes in between
                                            // the delete and the insert
                                            collection
                                                .delete_many(
                                                    doc! {"account_id": player.account_id as i64},
                                                    None,
                                                )
                                                .await
                                                .unwrap();
                                            if stats.len() > 0 {
                                                collection.insert_many(stats, None).await.unwrap();
                                            }
                                        }
                                        None => {}
                                    };
                                }
                                None => {
                                    //
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!(
                                "Got an error retrieving detailed stats for player {}",
                                player.account_id
                            );
                        }
                    }
                }
            }
        });
    }

    // Go forever
    x.await;
}
