use std::iter::Map;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use reqwest::{Error, Url};
use reqwest::cookie::{CookieStore, Jar};
use reqwest::header::USER_AGENT;
use serde_json::from_value;

static POE_ENDPOINT: &str = "https://api.pathofexile.com";
static TRADE_ENDPOINT: &str = "http://www.pathofexile.com/api/trade/exchange";

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
    amount: u32
}

#[derive(Deserialize, Debug)]
struct TradeExchangeItem {
    currency: String,
    amount: u32,
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

    let response = client
        .post(request_url.clone())
        .header("User-Agent", "KalandraKapital")
        .json(&trade_request)
        .send()
        .await?;

    let result_text = response.text().await?;

    let response: TradeExchangeResponse = serde_json::from_str(&result_text).unwrap();

    let result: Vec<TradeExchangeResult> = response.result.into_iter().filter_map(|(_, value)| serde_json::from_value(value).ok()).collect();

    Ok(result)
}

fn main() {
    use tokio::runtime::Runtime;

    let rt = Runtime::new().unwrap();
    //let result = rt.block_on(async { get_leagues().await.unwrap() });

    //// Ensure that our selected league is available
    //if result.leagues.iter().find(|league| league.id == SELECTED_LEAGUE).is_some() {
    //    println!("Selected league: {}", SELECTED_LEAGUE);
    //    println!("POESESSID = {}", std::env::var("POESESSID").unwrap());
    //} else {
    //    panic!("Selected league is not available!")
    //}

    let result = rt.block_on(async { get_bulk_results("mirror", "scrap").await.unwrap() });

    println!("{:#?}", result);
}
