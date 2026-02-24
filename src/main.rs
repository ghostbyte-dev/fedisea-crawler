use reqwest::Url;
use serde::Deserialize;

#[tokio::main]
async fn main() {
    println!("Hello, world!");
    fetch_instance("mastodon.social").await;
}

async fn fetch_instance(instance: &str) {
    let well_known = match fetch_well_known(instance).await {
        Ok(well_known) => well_known,
        Err(_) => return,
    };

    println!("Got instance: {}", instance);
    println!("Got well known rel: {}", well_known.links[0].rel);

    let nodeinfo = match fetch_nodeinfo(&*well_known.links[0].href).await {
        Ok(nodeinfo) => nodeinfo,
        Err(_) => return,
    };

    println!("Got software: {}", nodeinfo.software.name);

    let peers = match fetch_peers(instance).await {
        Ok(peers) => peers,
        Err(_) => return,
    };

    println!("Got peers length: {}", peers.len());
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

    let res: WellKnown = reqwest::get(url).await?.error_for_status()?.json().await?;
    Ok(res)
}

async fn fetch_peers(instance: &str) -> Result<Vec<String>, anyhow::Error> {
    let url = format!("https://{}/api/v1/instance/peers", instance);
    let url = Url::parse(&*url)?;

    let res: Vec<String> = reqwest::get(url).await?.error_for_status()?.json().await?;
    Ok(res)
}

#[derive(Deserialize)]
struct Nodeinfo {
    software: Software
}

#[derive(Deserialize)]
struct Software {
    name: String,
    version: String
}

async fn fetch_nodeinfo(url: &str) -> Result<Nodeinfo, anyhow::Error> {
    let url = Url::parse(url)?;

    let res: Nodeinfo = reqwest::get(url).await?.error_for_status()?.json().await?;
    Ok(res)
}