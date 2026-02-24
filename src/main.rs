use reqwest::Url;
use serde::Deserialize;
use std::collections::{HashSet, VecDeque};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::time::{Duration, SystemTime};

#[tokio::main]
async fn main() {
    let now = SystemTime::now();

    let seed = "pixelfed.social";
    let mut to_visit: VecDeque<String> = VecDeque::new();
    let mut found_urls: HashSet<String> = HashSet::new();
    to_visit.push_back(seed.to_string());
    found_urls.insert(seed.to_string());
    let mut index = 0;

    let path = Path::new("data.txt");
    let display = path.display();
    let mut file = match File::create(&path) {
        Err(why) => panic!("couldn't create {}: {}", display, why),
        Ok(file) => file,
    };

    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .connect_timeout(Duration::from_secs(2))
        .build()
        .expect("reqwest client failed");

    while let Some(url) = to_visit.pop_front() {
        if index > 200 {
            break;
        }
        let peers = match fetch_instance(url, &mut file, &http_client).await {
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

    match now.elapsed() {
        Ok(elapsed) => {
            println!("{} sec", elapsed.as_secs());
        }
        Err(e) => {
            println!("Great Scott! {e:?}");
        }
    }
}

async fn fetch_instance(
    instance: String,
    file: &mut File,
    http_client: &reqwest::Client,
) -> Result<Vec<String>, anyhow::Error> {
    println!("Fetching instance: {}", instance);
    let well_known = fetch_well_known(instance.clone(), http_client).await?;

    println!("Got well known rel: {}", well_known.links[0].rel);

    let nodeinfo = fetch_nodeinfo(&*well_known.links[0].href, http_client)
        .await
        .ok();

    if let Some(nodeinfo) = nodeinfo {
        println!("Got software: {}", nodeinfo.software.name);
        save_data(instance.clone(), nodeinfo, file);
    }

    let peers = fetch_peers(instance, http_client).await?;

    println!("Got peers length: {}", peers.len());
    Ok(peers)
}

fn save_data(instance: String, nodeinfo: Nodeinfo, file: &mut File) {
    writeln!(
        file,
        "instance: {}, {}: {}",
        instance, nodeinfo.software.name, nodeinfo.software.version
    )
    .expect("Failed to save to file");
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

async fn fetch_well_known(
    instance: String,
    http_client: &reqwest::Client,
) -> Result<WellKnown, anyhow::Error> {
    let url = format!("https://{}/.well-known/nodeinfo", instance,);
    let url = Url::parse(&*url)?;

    let res: WellKnown = http_client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(res)
}

async fn fetch_peers(
    instance: String,
    http_client: &reqwest::Client,
) -> Result<Vec<String>, anyhow::Error> {
    let url = format!("https://{}/api/v1/instance/peers", instance);
    let url = Url::parse(&*url)?;

    let res: Vec<String> = http_client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
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

async fn fetch_nodeinfo(
    url: &str,
    http_client: &reqwest::Client,
) -> Result<Nodeinfo, anyhow::Error> {
    let url = Url::parse(url)?;

    let res: Nodeinfo = http_client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(res)
}
