use reqwest::Url;
use serde::Deserialize;
use std::collections::{HashSet, VecDeque};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::time::{Duration, SystemTime};
use tokio::sync::mpsc;
use tokio_stream::wrappers::{ReceiverStream, UnboundedReceiverStream};
use futures::StreamExt;

#[tokio::main]
async fn main() {
    let now = SystemTime::now();

    let seed = "techhub.social";
    let mut found_urls: HashSet<String> = HashSet::new();
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

    let (tx, rx) = mpsc::unbounded_channel::<String>();

    tx.send(seed.to_string()).expect("send failed");

    let mut stream = UnboundedReceiverStream::new(rx)
        .map(|url: String| {
            let client = http_client.clone();
            async move {
                (url.clone(), fetch_instance(url, &client).await)
            }
        })
        .buffer_unordered(20);

    while let Some((url, result)) = stream.next().await {
        if index >= 200 {
            break;
        }

        match result {
            Ok(peers) => {
                save_data(url, &mut file);

                for peer in peers {
                    if found_urls.insert(peer.clone()) {
                        let _ = tx.send(peer);
                    }
                }
                index += 1;
                println!("Processed: {}/200 | Queue hidden in channel", index);
            }
            Err(e) => {
                eprintln!("Error fetching {}: {}", url, e);
            }
        }
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
    http_client: &reqwest::Client,
) -> Result<Vec<String>, anyhow::Error> {
    println!("Fetching instance: {}", instance);
    let well_known = fetch_well_known(instance.clone(), http_client).await?;

    let nodeinfo = fetch_nodeinfo(&*well_known.links[0].href, http_client)
        .await
        .ok();

    let peers = fetch_peers(instance, http_client).await?;
    Ok(peers)
}

fn save_data(instance: String, file: &mut File) {
    writeln!(
        file,
        "instance: {}",
        instance
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
