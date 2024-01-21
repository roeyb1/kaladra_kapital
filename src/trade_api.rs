use std::cmp::min;
use std::fs;
use std::fs::OpenOptions;
use std::io::prelude::*;
use log::warn;
use reqwest::{Error, Url};
use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;
use crate::{build_client, ItemPrice, SELECTED_LEAGUE};
use crate::CurrencyOrbType::{Chaos, Divine};

static EXCHANGE_ENDPOINT: &str = "https://www.pathofexile.com/api/trade/exchange";
static SEARCH_ENDPOINT: &str = "https://www.pathofexile.com/api/trade/search";
static FETCH_ENDPOINT: &str = "https://www.pathofexile.com/api/trade/fetch";

// #todo: remove pub
#[derive(Serialize, Clone, Debug)]
pub enum TradeStatus {
    online,
    onlineleague,
    any,
}

#[derive(Serialize, Debug)]
struct TradeExchangeQuery {
    status: TradeStatus,
    have: Vec<String>,
    want: Vec<String>,
    minimum: Option<u32>,
}

#[derive(Serialize, Debug)]
enum TradeSortDirection {
    asc,
    desc
}

#[derive(Serialize, Debug)]
struct TradeSortMode {
    #[serde(skip_serializing_if = "Option::is_none")]
    have: Option<TradeSortDirection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    price: Option<TradeSortDirection>
}

impl TradeSortMode {
    fn have(dir: TradeSortDirection) -> Self {
        Self { have: Some(dir), price: None }
    }
    fn price(dir: TradeSortDirection) -> Self {
        Self { have: None, price: Some(dir) }
    }
}

#[derive(Serialize, Debug)]
struct TradeExchangeRequest {
    query: TradeExchangeQuery,
    sort: TradeSortMode,
    engine: String,
}

// #todo: rename
#[derive(Deserialize, Debug)]
struct TradeExchangeExchange {
    currency: String,
    amount: f32
}

#[derive(Deserialize, Debug)]
struct TradeExchangeItem {
    currency: String,
    amount: f32,
    stock: u32
}

#[derive(Deserialize, Debug)]
struct TradeExchangeOffer {
    exchange: TradeExchangeExchange,
    item: TradeExchangeItem
}

#[derive(Deserialize, Debug)]
struct TradeExchangeListing {
    offers: Vec<TradeExchangeOffer>
}

#[derive(Deserialize, Debug)]
struct TradeExchangeResult {
    listing: TradeExchangeListing
}

#[derive(Deserialize, Debug)]
struct TradeExchangeResponse {
    result: serde_json::Map<String, serde_json::Value>,
    total: u32
}

impl From<TradeExchangeResponse> for Vec<TradeExchangeResult> {
    fn from(response: TradeExchangeResponse) -> Self {
        response.result.into_iter().filter_map(|(_, value)| serde_json::from_value(value).ok()).collect()
    }
}

async fn get_bulk_results(have: &str, want: &str) -> Result<Vec<TradeExchangeResult>, Error> {
    // check the cache before submitting a web request to limit API calls.
    let cache_string =  fs::read_to_string("cache/requests.csv");
    match cache_string {
        Ok(cache_data) => {
            let havewant_pair = format!("{}->{}", have, want);
            for line in cache_data.split("\n") {
                let split_line: Vec<&str> = line.split(",").collect();
                let key = split_line.get(0).unwrap();

                if *key == havewant_pair {
                    let value = split_line.get(1).unwrap();
                    log::debug!("Request for {{have: {}, want: {}}} found in cache!", have, want);
                    let response: TradeExchangeResponse = serde_json::from_str(
                        std::str::from_utf8(&*base64::decode(value).unwrap())
                            .unwrap())
                        .unwrap();
                    let result: Vec<TradeExchangeResult> = response.into();
                    return Ok(result);
                }
            }
        }
        _ => {}
    }

    let request_url = format!("{}/{}", EXCHANGE_ENDPOINT, SELECTED_LEAGUE).parse::<Url>().unwrap();

    let client = build_client(&request_url)?;

    let trade_request = TradeExchangeRequest {
        query: TradeExchangeQuery {
            status: TradeStatus::online,
            have: vec![have.to_string()],
            want: vec![want.to_string()],
            minimum: None,
        },
        sort: TradeSortMode::have(TradeSortDirection::asc),
        engine: "new".to_string(),
    };

    log::debug!("Submitting request to trade API...");

    let response = client
        .post(request_url.clone())
        .header("User-Agent", "KalandraKapital/1.0")
        .json(&trade_request)
        .send()
        .await?;

    let result_text = response.text().await?;

    let mut cache_file = OpenOptions::new()
        .write(true)
        .append(true)
        .open("cache/requests.csv")
        .unwrap();
    writeln!(cache_file, "{}->{},{}", have, want, base64::encode(&result_text)).expect("Unable to write request data to cache");

    let response: TradeExchangeResponse = serde_json::from_str(&result_text).unwrap();

    let result: Vec<TradeExchangeResult> = response.into();

    Ok(result)
}

#[derive(Serialize, Clone, Debug)]
pub struct TradeSearchFilter {

}

#[derive(Serialize, Clone, Debug)]
pub struct TradeSearchQuery {
    pub status: TradeStatus,
    #[serde(rename="type")]
    pub item_type: String,
    pub filters: Vec<TradeSearchFilter>,
}

#[derive(Serialize, Debug)]
struct TradeSearchRequest {
    query: TradeSearchQuery,
    sort: TradeSortMode,
}

#[derive(Deserialize, Debug)]
struct TradeSearchPrice {
    amount: f32,
    currency: String,
}

#[derive(Deserialize, Debug)]
struct TradeSearchListing {
    price: TradeSearchPrice
}

#[derive(Deserialize, Debug)]
struct TradeSearchItem {
    // #todo: do we need this?
}

#[derive(Deserialize, Debug)]
struct TradeSearchResult {
    id: String,
    listing: TradeSearchListing,
    item: TradeSearchItem,
}

#[derive(Deserialize, Debug)]
struct TradeFetchResponse {
    result: Vec<TradeSearchResult>,
}

#[derive(Deserialize, Debug)]
struct TradeSearchResponse {
    id: String,
    result: Vec<String>,
    total: u32
}

async fn get_search_results(query: TradeSearchQuery) -> Result<Vec<TradeSearchResult>, Error> {
    let request_url = format!("{}/{}", SEARCH_ENDPOINT, SELECTED_LEAGUE).parse::<Url>().unwrap();

    let client = build_client(&request_url)?;

    let trade_request = TradeSearchRequest {
        query: query,
        sort: TradeSortMode::price(TradeSortDirection::asc)
    };

    println!("{}", serde_json::to_string(&trade_request).unwrap());

    log::debug!("Submitting request to trade API...");

    let response = client
        .post(request_url.clone())
        .header("User-Agent", "KalandraKapital/1.0")
        .json(&trade_request)
        .send()
        .await?;

    let response: TradeSearchResponse = response.json().await?;

    let fetch_results = response.result[0..10].join(",");

    let response = client
        .get(format!("{FETCH_ENDPOINT}/{fetch_results}?q={}", response.id))
        .header("User-Agent", "KalandraKapital/1.0")
        .send()
        .await?;

    // #todo: cache

    let response: TradeFetchResponse = response.json().await?;

    Ok(response.result)
}

pub fn create_request_cache() -> Result<(), String> {
    match fs::read_dir("cache") {
        Ok(_) => { Ok(()) }
        Err(_) => {
            match fs::create_dir("cache") {
                Ok(_) => Ok(()),
                Err(e) => Err(format!("Failed to create request cache directory ({})", e))
            }
        }
    }
}

// #todo: this function should return more complex pricing data after doing a full analysis (min/average at the very least)
pub fn get_bulk_pricing(have: &str, want: &str) -> ItemPrice {
    let rt = Runtime::new().unwrap();
    let result = rt.block_on(async { get_bulk_results(have, want).await.unwrap() });

    let mut min_price = f32::MAX;
    for listing in result {
        let exchange = &listing.listing.offers[0].exchange;
        let item = &listing.listing.offers[0].item;

        let amount_payed = exchange.amount;
        let amount_received = item.amount;

        min_price = f32::min(amount_payed / amount_received, min_price);
    }

    ItemPrice::new(min_price, Divine)
}

pub fn get_search_pricing(query: TradeSearchQuery) -> Option<ItemPrice> {
    let rt = Runtime::new().unwrap();
    let query_results = rt.block_on(async { get_search_results(query).await.unwrap() });

    let mut prices: Vec<ItemPrice> = Vec::new();
    for result in query_results {
        println!("{} {}", result.listing.price.amount, result.listing.price.currency);
        match result.listing.price.currency.as_str() {
            "divine" => prices.push(ItemPrice::new(result.listing.price.amount, Divine)),
            "chaos" => prices.push(ItemPrice::new(result.listing.price.amount, Chaos)),
            _ => {}
        }
    }

    prices.iter().min().cloned()
}