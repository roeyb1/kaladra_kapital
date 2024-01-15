mod trade_api;

use std::fs;
use std::ptr::addr_of_mut;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use reqwest::{Error, Url};
use reqwest::cookie::Jar;
use tokio::runtime::Runtime;

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

static CHAOS_TO_DIVINE_RATIO: f32 = 215.;

#[derive(Debug)]
struct ChaosOrbPrice {
    amount: f32
}

#[derive(Debug)]
struct DivineOrbPrice {
    amount: f32
}

impl From<ChaosOrbPrice> for DivineOrbPrice {
    fn from(value: ChaosOrbPrice) -> Self {
        DivineOrbPrice { amount: value.amount / CHAOS_TO_DIVINE_RATIO }
    }
}

impl From<DivineOrbPrice> for ChaosOrbPrice {
    fn from(value: DivineOrbPrice) -> Self {
        ChaosOrbPrice { amount: value.amount * CHAOS_TO_DIVINE_RATIO }
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
    let result_as_chaos: ChaosOrbPrice = result.into();

    log::info!("It costs {} divine orbs per 1-socket resonator", result.amount);
    log::info!("It costs {} chaos orbs per 1-socket resonator", result_as_chaos.amount);
}
