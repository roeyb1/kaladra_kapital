use std::fs;
use std::fs::OpenOptions;
use std::sync::Arc;
use std::io::prelude::*;
use serde::{Deserialize, Serialize};
use reqwest::{Error, Url};
use reqwest::cookie::Jar;
use tokio::runtime::Runtime;

static POE_ENDPOINT: &str = "https://api.pathofexile.com";
static TRADE_ENDPOINT: &str = "https://www.pathofexile.com/api/trade/exchange";

// #todo: this should be stored in a config file somewhere
static SELECTED_LEAGUE: &str = "Affliction";

#[derive(Deserialize, Debug)]
struct League {
    id: String,
}

#[derive(Deserialize, Debug)]
struct Leagues {
    leagues: Vec<League>
}

#[derive(Serialize, Debug)]
enum TradeStatus {
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
    have: Option<TradeSortDirection>
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

fn build_client(request_url: &Url) -> Result<reqwest::Client, Error> {
    // #todo: POESESSID should be stored and read from a config file. for now just use env vars to keep this private during development
    let cookie = format!("{}={}", "POESESSID", std::env::var("POESESSID").unwrap());
    let jar = Arc::new(Jar::default());
    jar.add_cookie_str(&cookie, &request_url);

    reqwest::Client::builder()
        .cookie_provider(Arc::clone(&jar))
        .cookie_store(true)
        .build()
}

async fn get_leagues() -> Result<Leagues, Error> {
    let request_url = format!("{}/league", POE_ENDPOINT).parse::<Url>().unwrap();
    let client = build_client(&request_url)?;
    let response = client.get(request_url).send().await?;

    let result: Leagues = response.json().await?;

    Ok(result)
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

    let request_url = format!("{}/{}", TRADE_ENDPOINT, SELECTED_LEAGUE).parse::<Url>().unwrap();

    let client = build_client(&request_url)?;

    let trade_request = TradeExchangeRequest {
        query: TradeExchangeQuery {
            status: TradeStatus::online,
            have: vec![have.to_string()],
            want: vec![want.to_string()],
            minimum: None,
        },
        sort: TradeSortMode { have: Some(TradeSortDirection::asc) },
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

// #todo: this function should return more complex pricing data after doing a full analysis (min/average at the very least)
fn get_bulk_pricing(have: &str, want: &str) -> f32 {
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

    min_price
}

fn setup_environment() -> Result<(), String> {
    match fs::read_dir("cache") {
        Ok(_) => { Ok(()) }
        Err(_) => {
            match fs::create_dir("cache") {
                Ok(_) => { Ok(()) }
                Err(e) => { Err(format!("Failed to create request cache directory ({})", e))}
            }
        }
    }
}

fn main() {
    env_logger::init();

    match setup_environment() {
        Err(error) => {
            log::error!("Critical setup failure: {}", error);
            return;
        },
        _ => log::info!("Setup complete")
    };

    let rt = Runtime::new().unwrap();
    //let result = rt.block_on(async { get_leagues().await.unwrap() });

    //// Ensure that our selected league is available
    //if result.leagues.iter().find(|league| league.id == SELECTED_LEAGUE).is_some() {
    //    println!("Selected league: {}", SELECTED_LEAGUE);
    //    println!("POESESSID = {}", std::env::var("POESESSID").unwrap());
    //} else {
    //    panic!("Selected league is not available!")
    //}


    let result = get_bulk_pricing("divine", "serrated-fossil");

    log::info!("It costs {} divine orbs per serrated fossil", result);
}
