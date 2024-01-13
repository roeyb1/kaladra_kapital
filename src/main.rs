use serde::Deserialize;
use reqwest::Error;

#[derive(Deserialize, Debug)]
struct Test {

}

async fn get_test() -> Result<String, Error> {
    let request_url = format!("https://api.github.com/repos/{owner}/{repo}/stargazers",
                              owner = "rust-lang-nursery",
                              repo = "rust-cookbook");

    println!("{}", request_url);

    let response = reqwest::get(&request_url).await?;

    let result = response.json().await?;

    Ok(result)
}

fn main() {
    use tokio::runtime::Runtime;

    let rt = Runtime::new().unwrap();
    let result = rt.block_on(async { get_test().await.unwrap() });

    println!("{}", result);
}
