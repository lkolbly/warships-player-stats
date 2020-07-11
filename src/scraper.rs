//use serde::{Deserialize, Serialize};
//use serde_derive::{Deserialize, Serialize};
//use futures::future::{join_all, select_all};
//use futures::{join};
//use tokio::select;
//use futures::stream::{Stream};
//use futures::task::{Poll, Context};
//use std::pin::Pin;
//use itertools::Itertools;
//use itertools::CombinationsWithReplacement;
//use futures::stream::StreamExt;
//use futures::stream::TryStreamExt;
use stream_throttle::{ThrottleRate, ThrottlePool};
use stream_throttle::ThrottledStream;
use futures::future::FutureExt;
//use futures::stream::FuturesUnordered;
use sha2::{Sha256, Digest};
use std::io::{Read, Write};
use std::sync::Arc;
//use thiserror::Error;
//use std::backtrace::Backtrace;
use tokio::fs::File;
use tokio::prelude::*;
use std::collections::HashMap;
//use indicatif::ProgressIterator;
use flate2::Compression;
use flate2::read::{GzEncoder, GzDecoder};
use futures::future::TryFutureExt;
use std::convert::TryInto;
use histogram::Histogram;
use std::sync::Mutex;

use crate::error::Error;
use crate::wows_data::*;
use crate::progress_logger::ProgressLogger;

pub struct WowsClient {
    application_id: String,
    client: reqwest::Client,
    throttle_pool: ThrottlePool,
    logger: Arc<Mutex<ProgressLogger>>,
    //cache: Arc<sled::Db>,
}

impl WowsClient {
    pub fn new(application_id: &str) -> WowsClient {
        let client = reqwest::Client::new();
        WowsClient {
            application_id: application_id.to_string(),
            client: client,
            throttle_pool: ThrottlePool::new(ThrottleRate::new(1, std::time::Duration::new(0, 50_000_000))),
            logger: Arc::new(Mutex::new(ProgressLogger::new("api_requests"))),
            //cache: Arc::new(sled::open("api_cache.sled").expect("Unable to create sled database")),
        }
    }

    pub fn fork(&self) -> WowsClient {
        WowsClient {
            application_id: self.application_id.to_string(),
            client: self.client.clone(),
            throttle_pool: self.throttle_pool.clone(),
            logger: self.logger.clone(),
            //cache: self.cache.clone(),
        }
    }

    fn hash(&self, uri: &str, params: &[(&str, &str)]) -> String {
        let mut s = uri.to_string();
        for p in params.iter() {
            s.push_str("_");
            s.push_str(p.0);
            s.push_str("_");
            s.push_str(p.1);
        }
        hex::encode(sha2::Sha256::digest(s.as_bytes()))
    }

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

    async fn request<T: serde::de::DeserializeOwned>(&self, uri: &str, params: &[(&str, &str)]) -> Result<T, Error> {
        let hash = self.hash(uri, params);
        let cache_path = format!("api_cache/{}", hash);
        let compressed_cache_path = format!("api_cache/{}.gz", hash);

        /*let cl = || {
            let body = self.cache.get(&hash)?;
            if let Some(body) = body {
                let body = self.decompress(&body)?;
                let s = std::str::from_utf8(body.as_ref())?;
                let res = serde_json::from_str(s)?;
                Ok(res)
            } else {
                Err(Error::CacheEntryNotFound)
            }
        };
        //let c_val = cl();
        match cl() {
            Ok(x) => { return Ok(x); }
            Err(_) => { /* Fall through */ }
        }
        /*let body = match self.cache.get(&hash) {
            Ok(Some(x)) => { Some(x) },
            Err(_) => None,
        }?;
        let s = match std::str::from_utf8(body.as_ref()) {
            Ok(x) => { Some(x) },
            Err(_) => None,
        }?;
        let res = match serde_json::from_str(s) {
            Ok(x) => { Some(x) },
            Err(_) => None,
        }?;
        return res;*/

        async {
            let mut f = File::open(&compressed_cache_path).await?;
            let mut body = vec!();
            f.read_to_end(&mut body).await?;
            let mut gz = GzDecoder::new(&body[..]);

            let mut body = String::new();
            gz.read_to_string(&mut body)?;
            let res = serde_json::from_str(&body)?;
            self.cache.insert(&hash, self.compress(body.as_bytes())?).expect("Error adding key to sled cache");
            Ok::<T, Error>(res)
        }.or_else(|_| async {
            // Try the uncompressed path
            let mut f = File::open(&cache_path).await?;
            let mut body = String::new();
            f.read_to_string(&mut body).await?;
            let res = serde_json::from_str(&body)?;
            self.cache.insert(&hash, self.compress(body.as_bytes())?).expect("Error adding key to sled cache");
            Ok::<T, Error>(res)
        }).or_else(|_| async {*/
            // Actually go fetch the URL
            let mut params = params.to_vec();
            params.push(("application_id", self.application_id.as_str()));

            let body = self.throttle_pool.queue().then(|_| async {
                self.client.get(uri)
                    .form(&params)
                    .send()
                    .await?
                    .text()
                    .await
            }).await?;
            match serde_json::from_str(&body) {
                Ok(x) => {
                    {
                        let mut logger = self.logger.lock().unwrap();
                        logger.increment(1);
                    }
                    //self.cache.insert(&hash, self.compress(body.as_bytes())?).expect("Error adding key to sled cache");

                    /*let mut f = File::create(&compressed_cache_path).await.expect("Unable to create file");
                    let mut gz = GzEncoder::new(body.as_bytes(), Compression::fast());
                    let mut body = vec!();
                    let count = gz.read_to_end(&mut body);
                    f.write_all(&body).await.expect("Unable to fill cache file");*/
                    Ok(x)
                }
                Err(e) => {
                    //println!("Received error parsing result!");
                    Err(Error::from(e))
                }
            }
    //}).await


            /*match File::open(&compressed_cache_path).await {
            Ok(mut f) => {
                let mut body = vec!();
                match f.read_to_end(&mut body).await {
                    Ok(_) => {
                        let mut gz = GzDecoder::new(&body[..]);
                        let mut body = String::new();
                        match gz.read_to_string(&mut body) {
                            Ok(_) => {
                                match serde_json::from_str(&body) {//.expect(&format!("Unable to parse cached reply '{}'", cache_path));
                                    Ok(reply) => {
                                        return Ok(reply);
                                    },
                                    Err(_) => {
                                        // Failed to parse, fall through
                                    }
                                }
                            }
                            Err(_) => {
                                // Failed to decompress cache, fall through
                            }
                        }
                    }
                    Err(_) => {
                        // Failed to read compressed cache file, fall through
                    }
                }
            }
            Err(_) => {
                // Try the uncompressed version
                match File::open(&cache_path).await {
                    Ok(mut f) => {
                        let mut body = String::new();
                        match f.read_to_string(&mut body).await {
                            Ok(_) => {
                                match serde_json::from_str(&body) {//.expect(&format!("Unable to parse cached reply '{}'", cache_path));
                                    Ok(reply) => {
                                        return Ok(reply);
                                    },
                                    Err(_) => {
                                        // Failed to parse, fall through
                                    }
                                }
                            }
                            Err(_) => {
                                // Fall through
                            }
                        }
                    }
                    Err(_) => {
                        // Fall through
                    }
                }
            }
        }

        let mut params = params.to_vec();
        params.push(("application_id", self.application_id.as_str()));

        let body = self.throttle_pool.queue().then(|_| async {
            self.client.get(uri)
                .form(&params)
                .send()
                .await?
                .text()
                .await
        }).await?;
        match serde_json::from_str(&body) {
            Ok(x) => {
                let mut f = File::create(&compressed_cache_path).await.expect("Unable to create file");
                let mut gz = GzEncoder::new(body.as_bytes(), Compression::fast());
                let mut body = vec!();
                let count = gz.read_to_end(&mut body);
                f.write_all(&body).await.expect("Unable to fill cache file");
                Ok(x)
            }
            Err(e) => {
                //println!("Received error parsing result!");
                Err(Error::from(e))
            }
        }*/
    }

    async fn list_players_helper(&self, search: &str) -> Result<Vec<PlayerRecord>, Error> {
        let uri = "https://api.worldofwarships.com/wows/account/list/";
        let params = [
            ("search", search),
        ];
        let reply: GenericReply<Vec<PlayerRecord>> = self.request(uri, &params).await?;
        Ok(reply.data)
    }

    pub async fn list_players(&self, search: &str) -> Result<Vec<PlayerRecord>, Error> {
        let mut searches: Vec<String> = vec![search.to_string()];
        let mut i = 0;
        let mut result = vec!();
        while i < searches.len() {
            let mut reply = self.list_players_helper(&searches[i]).await?;
            if reply.len() == 100 {
                // Gotta go re-request for each sub-uri
                let chars = vec!['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z'];
                //let mut result = vec!();
                for c in chars.iter() {
                    let mut search = searches[i].clone();
                    search.push(*c);
                    searches.push(search);
                    //result.append(&mut self.list_players_helper(&search).await?);
                }
                //return Ok(result);
            } else {
                result.append(&mut reply);
            }
            i += 1;
        }
        Ok(result)
    }

    pub async fn get_detailed_stats(&self, account_id: u64) -> Result<HashMap<String, Option<Vec<DetailedStatTypes>>>, Error> {
        let uri = "https://api.worldofwarships.com/wows/ships/stats/";
        let s = format!("{}", account_id);
        let params = [
            ("account_id", s.as_str()),
        ];
        //let reply: DetailedStatsReply = self.request(uri, &params[..]).await?;
        let reply: GenericReply<HashMap<String, Option<Vec<DetailedStatTypes>>>> = self.request(uri, &params[..]).await?;
        Ok(reply.data)
    }

    pub async fn get_ship_info(&self, ship_id: u64) -> Result<ShipInfo, Error> {
        let uri = "https://api.worldofwarships.com/wows/encyclopedia/ships/";
        let s = format!("{}", ship_id);
        let params = [
            ("ship_id", s.as_str()),
        ];
        let reply: GenericReply<ShipInfo> = self.request(uri, &params[..]).await?;
        Ok(reply.data)
    }

    pub async fn enumerate_ships(&self) -> Result<HashMap<u64, ShipInfo>, Error> {
        let uri = "https://api.worldofwarships.com/wows/encyclopedia/ships/";
        let mut result: HashMap<_, ShipInfo> = HashMap::new();
        let params = [
            ("page_no", "1"),
        ];
        let reply: GenericReply<HashMap<String, Option<ShipInfo>>> = self.request(uri, &params[..]).await?;
        for (k,v) in reply.data.iter() {
            match v {
                Some(v) => { result.insert(k.parse().unwrap(), v.clone()); }
                None => {}
            }
        }
        for page in 2..reply.meta.page_total.unwrap()+1 {
            let page = format!("{}", page);
            let params = [
                ("page_no", page.as_str()),
            ];
            let reply: GenericReply<HashMap<String, Option<ShipInfo>>> = self.request(uri, &params[..]).await?;
            for (k,v) in reply.data.iter() {
                match v {
                    Some(v) => { result.insert(k.parse().unwrap(), v.clone()); }
                    None => {}
                }
            }
        }
        Ok(result)
    }
}
