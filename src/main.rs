mod trade_api;
mod strat;

use std::cmp::Ordering;
use std::fs;
use std::ptr::addr_of_mut;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use reqwest::{Error, Url};
use reqwest::cookie::Jar;
use tokio::runtime::Runtime;
use crate::trade_api::{get_search_pricing, TradeSearchQuery};

static POE_ENDPOINT: &str = "https://api.pathofexile.com";

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

fn setup_environment() -> Result<(), String> {
    trade_api::create_request_cache()
}

static CHAOS_TO_DIVINE_RATIO: f32 = 215.;

#[derive(Debug, Clone, Copy, PartialEq)]
enum CurrencyOrbType {
    Chaos,
    Divine
}

#[derive(Debug, Clone, Copy)]
struct ItemPrice {
    amount: f32,
    currency_orb: CurrencyOrbType
}

impl ItemPrice {
    fn new(amount: f32, currency_orb: CurrencyOrbType) -> Self {
        ItemPrice { amount, currency_orb }
    }

    fn as_chaos(&self) -> f32 {
        match self.currency_orb {
            CurrencyOrbType::Divine => self.amount * CHAOS_TO_DIVINE_RATIO,
            CurrencyOrbType::Chaos => self.amount,
        }
    }

    fn as_divine(&self) -> f32 {
        match self.currency_orb {
            CurrencyOrbType::Divine => self.amount,
            CurrencyOrbType::Chaos => self.amount / CHAOS_TO_DIVINE_RATIO,
        }
    }
}

impl PartialEq for ItemPrice {
    fn eq(&self, other: &Self) -> bool {
        self.as_chaos() == other.as_chaos()
    }
}

impl Eq for ItemPrice {}

impl PartialOrd for ItemPrice {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.as_chaos().partial_cmp(&other.as_chaos())
    }
}

impl Ord for ItemPrice {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_chaos().total_cmp(&other.as_chaos())
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

    let result = trade_api::get_bulk_pricing("divine", "serrated-fossil");

    log::info!("It costs {:?} divine orbs per serrated fossil", result.amount);

    let result = trade_api::get_bulk_pricing("divine", "primitive-chaotic-resonator");

    log::info!("It costs {} divine orbs per 1-socket resonator", result.as_divine());
    log::info!("It costs {} chaos orbs per 1-socket resonator", result.as_chaos());

    let query = TradeSearchQuery {
        status: trade_api::TradeStatus::online,
        item_type: "Vivid Vulture".to_string(),
        filters: vec![],
    };

    let result = trade_api::get_search_pricing(query.clone()).unwrap();

    log::info!("It costs {} divine orbs for a {}", result.as_divine(), query.item_type);
}
