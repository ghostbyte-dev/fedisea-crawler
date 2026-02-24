mod client;
mod models;
mod storage;

use crate::client::fetch_instance;
use crate::storage::save_data;
use futures::StreamExt;
use std::collections::HashSet;
use std::fs::File;
use std::path::Path;
use std::time::{Duration, SystemTime};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;

#[tokio::main]
async fn main() {
    let now = SystemTime::now();

    let mut found_urls: HashSet<String> = HashSet::new();

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

    let seed = "mastodon.social";
    tx.send(seed.to_string()).expect("send failed");
    found_urls.insert(seed.to_string());

    let mut stream = UnboundedReceiverStream::new(rx)
        .map(|url: String| {
            let client = http_client.clone();
            async move { (url.clone(), fetch_instance(url, &client).await) }
        })
        .buffer_unordered(20);

    let mut index = 0;

    while let Some((url, result)) = stream.next().await {
        if index >= 200 {
            break;
        }

        match result {
            Ok(result_tuple) => {
                match result_tuple.0 {
                    Some(node_info) => save_data(url, node_info, &mut file),
                    None => (),
                }

                for peer in result_tuple.1 {
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
