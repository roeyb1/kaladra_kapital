use std::sync::Arc;
use serde::Deserialize;
use reqwest::{Error, Url};

static POESESSID_NAME: &str = "POESESSID";
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
    let cookie = format!("{}={}", POESESSID_NAME, std::env::var("POESESSID").unwrap());
    let jar = reqwest::cookie::Jar::default();
    jar.add_cookie_str(&cookie, &request_url);

    reqwest::Client::builder().cookie_provider(Arc::new(jar)).build()
}

async fn get_leagues() -> Result<Leagues, Error> {
    let request_url = format!("{}/league", POE_ENDPOINT).parse::<Url>().unwrap();
    let client = build_client(&request_url)?;
    let response = client.get(request_url).send().await?;

    let result: Leagues = response.json().await?;

    Ok(result)
}

fn main() {
    use tokio::runtime::Runtime;

    let rt = Runtime::new().unwrap();
    let result = rt.block_on(async { get_leagues().await.unwrap() });

    // Ensure that our selected league is available
    if result.leagues.iter().find(|league| league.id == SELECTED_LEAGUE).is_some() {
        println!("Selected league: {}", SELECTED_LEAGUE);
    } else {
        panic!("Selected league is not available!")
    }
}
