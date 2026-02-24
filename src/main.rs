use reqwest::Url;
use serde::Deserialize;

#[tokio::main]
async fn main() {
    println!("Hello, world!");
    let text = fetch_well_known("mastodon.social").await.expect("Failed to fetch");
    println!("href: {}", text.links[0].href);
    println!("rel: {}", text.links[0].rel);
}

#[derive(Deserialize)]
struct WellKnown {
    links: Vec<WellKnownElement>
}

#[derive(Deserialize)]
struct WellKnownElement {
    rel: String,
    href: String
}

async fn fetch_well_known(instance: &str) -> Result<WellKnown, anyhow::Error> {
    let url = format!(
        "https://{}/.well-known/nodeinfo",
        instance,
    );
    let url = Url::parse(&*url)?;

    let res: WellKnown = reqwest::get(url).await?.json().await?;
    Ok(res)
}