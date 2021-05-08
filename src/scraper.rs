use futures::future::FutureExt;
use std::collections::HashMap;
use std::convert::TryInto;
use std::sync::Arc;
use std::sync::Mutex;
use stream_throttle::{ThrottlePool, ThrottleRate};

use crate::error::Error;
use crate::progress_logger::ProgressLogger;
use crate::wows_data::*;

pub struct WowsClient {
    application_id: String,
    client: reqwest::Client,
    throttle_pool: ThrottlePool,
    logger: Arc<Mutex<ProgressLogger>>,
}

impl WowsClient {
    pub fn new(application_id: &str, request_period: u64) -> WowsClient {
        let client = reqwest::Client::new();
        WowsClient {
            application_id: application_id.to_string(),
            client: client,
            throttle_pool: ThrottlePool::new(ThrottleRate::new(
                1,
                std::time::Duration::new(0, request_period.try_into().unwrap()),
            )),
            logger: Arc::new(Mutex::new(ProgressLogger::new("api_requests"))),
        }
    }

    pub fn fork(&self) -> WowsClient {
        WowsClient {
            application_id: self.application_id.to_string(),
            client: self.client.clone(),
            throttle_pool: self.throttle_pool.clone(),
            logger: self.logger.clone(),
        }
    }

    async fn request<T: serde::de::DeserializeOwned>(
        &self,
        uri: &str,
        params: &[(&str, &str)],
    ) -> Result<T, Error> {
        let mut params = params.to_vec();
        params.push(("application_id", self.application_id.as_str()));

        let body = self
            .throttle_pool
            .queue()
            .then(|_| async {
                self.client
                    .get(uri)
                    .form(&params)
                    .send()
                    .await?
                    .text()
                    .await
            })
            .await
            .map_err(|e| Error::Http {
                err: e,
                url: uri.to_string(),
                params: params
                    .iter()
                    .map(|(a, b)| (a.to_string(), b.to_string()))
                    .collect(),
            })?;
        match serde_json::from_str(&body) {
            Ok(x) => {
                {
                    let mut logger = self.logger.lock().unwrap();
                    logger.increment(1);
                }
                Ok(x)
            }
            Err(e) => {
                // It's possible we may need to parse it as an error
                /*
                {"status":"error","error":{"code":504,"message":"SOURCE_NOT_AVAILABLE","field":null,"value":null}}
                */
                // and then retry or return that error or something
                Err(Error::HttpParse {
                    err: e,
                    url: uri.to_string(),
                    params: params
                        .iter()
                        .map(|(a, b)| (a.to_string(), b.to_string()))
                        .collect(),
                })
            }
        }
    }

    async fn list_players_helper(&self, search: &str) -> Result<Vec<PlayerRecord>, Error> {
        let uri = "https://api.worldofwarships.com/wows/account/list/";
        let params = [("search", search)];
        loop {
            let reply: GenericReply<Vec<PlayerRecord>> = self.request(uri, &params).await?;
            if let Some(data) = reply.data {
                return Ok(data);
            } else if reply
                .error
                .as_ref()
                .expect("Didn't get data, but expected error")
                .code
                != 504
            {
                return Err(Error::ApiError {
                    err: reply
                        .error
                        .expect("Didn't get data, but expected error")
                        .code,
                    url: uri.to_string(),
                    params: params
                        .iter()
                        .map(|(a, b)| (a.to_string(), b.to_string()))
                        .collect(),
                });
            }
        }
    }

    pub async fn list_players(&self, search: &str) -> Result<Vec<PlayerRecord>, Error> {
        let mut searches: Vec<String> = vec![search.to_string()];
        let mut i = 0;
        let mut result = vec![];
        while i < searches.len() {
            let mut reply = self.list_players_helper(&searches[i]).await?;
            if reply.len() == 100 {
                // Gotta go re-request for each sub-uri
                let chars = vec![
                    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p',
                    'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
                ];
                for c in chars.iter() {
                    let mut search = searches[i].clone();
                    search.push(*c);
                    searches.push(search);
                }
            } else {
                result.append(&mut reply);
            }
            i += 1;
        }
        Ok(result)
    }

    pub async fn get_detailed_stats(
        &self,
        account_id: u64,
    ) -> Result<HashMap<String, Option<Vec<DetailedStatTypes>>>, Error> {
        let uri = "https://api.worldofwarships.com/wows/ships/stats/";
        let s = format!("{}", account_id);
        let params = [("account_id", s.as_str())];
        let reply: GenericReply<HashMap<String, Option<Vec<DetailedStatTypes>>>> =
            self.request(uri, &params[..]).await?;
        match reply.data {
            Some(data) => Ok(data),
            None => Err(Error::DetailedStats {
                url: uri.to_string(),
                params: params
                    .iter()
                    .map(|(a, b)| (a.to_string(), b.to_string()))
                    .collect(),
                status: reply.status,
                meta: reply.meta,
                error: reply.error,
            }),
        }
    }

    pub async fn get_module_info(
        &self,
        module_ids: &[u64],
    ) -> Result<HashMap<u64, DetailedModuleInfo>, Error> {
        let uri = "https://api.worldofwarships.com/wows/encyclopedia/modules/";
        let module_ids: Vec<String> = module_ids.iter().map(|x| format!("{}", x)).collect();
        let module_ids = module_ids.join(",");
        let params = [("module_id", module_ids.as_str())];
        let reply: GenericReply<HashMap<String, DetailedModuleInfo>> =
            self.request(uri, &params[..]).await?;
        let mut result: HashMap<u64, DetailedModuleInfo> = HashMap::new();
        for (k, v) in reply.data.expect("Expected data for module info").iter() {
            result.insert(k.parse::<u64>().unwrap(), v.clone());
        }
        Ok(result)
    }

    pub async fn get_ship_info(&self, ship_id: u64) -> Result<ShipInfo, Error> {
        let uri = "https://api.worldofwarships.com/wows/encyclopedia/ships/";
        let s = format!("{}", ship_id);
        let params = [("ship_id", s.as_str())];
        let reply: GenericReply<ShipInfo> = self.request(uri, &params[..]).await?;
        Ok(reply.data.expect("Expected data for get_ship_info"))
    }

    pub async fn enumerate_ships(&self) -> Result<HashMap<u64, ShipInfo>, Error> {
        let uri = "https://api.worldofwarships.com/wows/encyclopedia/ships/";
        let mut result: HashMap<_, ShipInfo> = HashMap::new();
        let params = [("page_no", "1")];
        let reply: GenericReply<HashMap<String, Option<ShipInfo>>> =
            self.request(uri, &params[..]).await?;
        for (k, v) in reply
            .data
            .expect("Expected data for enumerate_ships")
            .iter()
        {
            match v {
                Some(v) => {
                    result.insert(k.parse().unwrap(), v.clone());
                }
                None => {}
            }
        }
        for page in 2..reply
            .meta
            .expect("Expected meta for enumerate_ships")
            .page_total
            .unwrap()
            + 1
        {
            let page = format!("{}", page);
            let params = [("page_no", page.as_str())];
            let reply: GenericReply<HashMap<String, Option<ShipInfo>>> =
                self.request(uri, &params[..]).await?;
            for (k, v) in reply.data.expect("Expected data for reply data").iter() {
                match v {
                    Some(v) => {
                        result.insert(k.parse().unwrap(), v.clone());
                    }
                    None => {}
                }
            }
        }
        Ok(result)
    }
}
