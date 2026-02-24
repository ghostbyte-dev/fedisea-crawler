use reqwest::Url;
use serde::Deserialize;
use std::collections::{HashSet, VecDeque};

#[tokio::main]
async fn main() {
    println!("Hello, world!");
    let seed = "mastodon.social";
    let mut to_visit: VecDeque<String> = VecDeque::new();
    let mut found_urls: HashSet<String> = HashSet::new();
    to_visit.push_back(seed.to_string());
    found_urls.insert(seed.to_string());
    let mut index = 0;

    while let Some(url) = to_visit.pop_front() {
        if (index > 10) {
            break;
        }
        let peers = match fetch_instance(url).await {
            Ok(peers) => peers,
            Err(_) => continue,
        };

        let missing: Vec<String> = peers
            .iter()
            .filter(|peer| !found_urls.contains(*peer))
            .cloned()
            .collect();

        to_visit.extend(missing.clone());
        found_urls.extend(missing);

        println!("queue size: {}", to_visit.len());
        index = index + 1;
    }
}

async fn fetch_instance(instance: String) -> Result<Vec<String>, anyhow::Error> {
    println!("Fetching instance: {}", instance);
    let well_known = fetch_well_known(instance.clone()).await?;

    println!("Got well known rel: {}", well_known.links[0].rel);

    let nodeinfo = fetch_nodeinfo(&*well_known.links[0].href).await.ok();

    if let Some(nodeinfo) = nodeinfo {
        println!("Got software: {}", nodeinfo.software.name);
    }

    let peers = fetch_peers(instance).await?;

    println!("Got peers length: {}", peers.len());
    Ok(peers)
}

#[derive(Deserialize)]
struct WellKnown {
    links: Vec<WellKnownElement>,
}

#[derive(Deserialize)]
struct WellKnownElement {
    rel: String,
    href: String,
}

async fn fetch_well_known(instance: String) -> Result<WellKnown, anyhow::Error> {
    let url = format!("https://{}/.well-known/nodeinfo", instance,);
    let url = Url::parse(&*url)?;

    let res: WellKnown = reqwest::get(url).await?.error_for_status()?.json().await?;
    Ok(res)
}

async fn fetch_peers(instance: String) -> Result<Vec<String>, anyhow::Error> {
    let url = format!("https://{}/api/v1/instance/peers", instance);
    let url = Url::parse(&*url)?;

    let res: Vec<String> = reqwest::get(url).await?.error_for_status()?.json().await?;
    Ok(res)
}

#[derive(Deserialize)]
struct Nodeinfo {
    software: Software,
}

#[derive(Deserialize)]
struct Software {
    name: String,
    version: String,
}

async fn fetch_nodeinfo(url: &str) -> Result<Nodeinfo, anyhow::Error> {
    let url = Url::parse(url)?;

    let res: Nodeinfo = reqwest::get(url).await?.error_for_status()?.json().await?;
    Ok(res)
}
