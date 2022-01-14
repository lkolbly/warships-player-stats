use itertools::*;
use mongodb::bson::doc;
use serde_derive::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tracing::*;

use crate::error::*;
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
            'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', '_',
        ];
        loop {
            for prefix in (0..3).map(|_| alphabet.iter()).multi_cartesian_product() {
                let prefix: String = prefix.iter().map(|c| *c).collect();
                alphabet_sender.send(prefix).await.log_and_drop_error(|e| {
                    error!("Couldn't send prefix through pipe, error {:?}", e);
                });
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
                let players = client.list_players(&prefix).await.map_or_else(
                    |e| {
                        error!(
                            "Error listing players on WoWS API for prefix {}: {:?}",
                            prefix, e
                        );
                        vec![]
                    },
                    |players| players,
                );

                // Lowercase the usernames
                let players: Vec<PlayerRecord> = players
                    .iter()
                    .map(|player| PlayerRecord {
                        nickname: player.nickname.to_lowercase(),
                        account_id: player.account_id,
                    })
                    .collect();

                // Send the players, if they exist
                if players.len() > 0 {
                    let collection = database.collection::<PlayerRecord>("playerids");
                    for player in players.iter() {
                        collection
                            .delete_many(doc! { "nickname": player.nickname.clone() }, None)
                            .await
                            .log_and_drop_error(|e| {
                                error!("Error deleting pre-existing player record, error: {:?}", e);
                            });
                    }

                    collection
                        .insert_many(players.clone(), None)
                        .await
                        .log_and_drop_error(|e| {
                            error!("Error adding player records to mongo: {:?}", e);
                        });

                    // TODO: Implement upserts to avoid race conditions & improve performance
                    /*for player in players.iter() {
                        collection
                            .update_one(
                                doc! { "nickname": player.nickname.clone() },
                                mongodb::options::UpdateModifications::Document(doc! { "nickname": player.nickname.clone(), "account_id": player.account_id as i64 }),
                                None,
                            )
                            .await
                            .log_and_drop_error(|e| {
                                error!("Error adding player record {:?} to mongo: {:?}", player, e);
                            });
                    }*/

                    player_sender.send(players).await.log_and_drop_error(|e| {
                        error!("Couldn't send player list, error: {:?}", e);
                    });
                } else {
                    debug!("No players for prefix {}", prefix);
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

                            if let Some((_player_id, stats)) = stats.iter().next() {
                                if let Some(stats) = stats {
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

                                    let collection =
                                        database.collection::<DetailedStatRecord>("playerstats");

                                    // TODO: This is a race condition, if a query for this account comes in between
                                    // the delete and the insert. This should be an upsert.
                                    collection
                                        .delete_many(
                                            doc! {"account_id": player.account_id as i64},
                                            None,
                                        )
                                        .await
                                        .log_and_drop_error(|e| {
                                            error!(
                                                "Couldn't delete statistics for account_id={}, error {:?}",
                                                player.account_id, e
                                            );
                                        });
                                    if stats.len() > 0 {
                                        collection
                                            .insert_many(stats, None)
                                            .await
                                            .log_and_drop_error(|e| {
                                                error!(
                                                    "Couldn't insert stats for account_id={}, error {:?}",
                                                    player.account_id, e
                                                );
                                            });
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            error!(
                                "Got an error {:?} retrieving detailed stats for player {}",
                                e, player.account_id
                            );
                        }
                    }
                }
            }
        });
    }

    // Go forever
    x.await.expect("Prefix generator should not have exited!");
}
