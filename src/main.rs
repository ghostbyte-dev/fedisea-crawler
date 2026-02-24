mod client;
mod models;
mod storage;

use crate::client::fetch_instance;
use crate::storage::save_data;
use futures::StreamExt;
use std::collections::HashSet;
use std::env;
use std::fs::File;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use dashmap::DashSet;
use dotenv::dotenv;
use sqlx::PgPool;
use tokio::sync::mpsc;
use tokio::time::timeout;
use tokio_stream::wrappers::UnboundedReceiverStream;

#[tokio::main]
async fn main() {
    let now = SystemTime::now();

    let found_urls = Arc::new(DashSet::new());

    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(4))
        .connect_timeout(Duration::from_secs(2))
        .build()
        .expect("reqwest client failed");

    let shared_client = Arc::new(http_client);
    dotenv().ok();
    let database_url = env::var("DB_CONNECTION_STRING").expect("DB_CONNECTION_STRING must be set");
    let postgres_client = PgPool::connect(&database_url).await.expect("connect to db failed");

    let (tx, rx) = mpsc::unbounded_channel::<String>();

    let seed = "pixelfed.social";
    tx.send(seed.to_string()).expect("send failed");
    found_urls.insert(seed.to_string());

    let mut db_set = tokio::task::JoinSet::new();

    let mut stream = UnboundedReceiverStream::new(rx)
        .map(|url: String| {
            let client = shared_client.clone();
            let visited_clone = found_urls.clone();
            let tx_discovery = tx.clone();

            async move {
                let result = fetch_instance(url.clone(), client).await;

                if let Ok(ref result_tuple) = result {
                    for peer in &result_tuple.1 {
                        if !peer.contains("troll") && visited_clone.insert(peer.clone()) {
                            let _ = tx_discovery.send(peer.clone());
                        }
                    }
                }

                (url, result)
            }
        })
        .buffer_unordered(20);

    let mut index = 0;

    while let Ok(Some((url, result))) = timeout(Duration::from_secs(30), stream.next()).await {
        if index >= 1000 {
            break;
        }

        match result {
            Ok(result_tuple) => {
                if let Some(node_info) = result_tuple.0 {
                    let pool_clone = postgres_client.clone();
                    db_set.spawn(async move {
                        save_data(url, node_info, &pool_clone).await;
                    });
                }

                index += 1;
                println!("Processed: {}/200 | Queue hidden in channel", index);
            }
            Err(e) => {
                eprintln!("Error fetching {}: {}", url, e);
            }
        }
    }

    println!("finish db updates");
    while let Some(_) = db_set.join_next().await {}
    println!("finished db updates");
    match now.elapsed() {
        Ok(elapsed) => {
            println!("{} sec", elapsed.as_secs());
        }
        Err(e) => {
            println!("Great Scott! {e:?}");
        }
    }
}
