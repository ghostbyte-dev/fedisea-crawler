mod client;
mod models;
mod storage;

use crate::client::fetch_instance;
use crate::storage::save_data;
use futures::StreamExt;
use std::env;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use dashmap::DashSet;
use dotenv::dotenv;
use sqlx::PgPool;
use tokio::sync::mpsc;
use tokio::time::timeout;
use tokio_stream::wrappers::{ReceiverStream};

#[tokio::main]
async fn main() {
    let now = SystemTime::now();

    let found_urls = Arc::new(DashSet::new());

    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .connect_timeout(Duration::from_secs(1))
        .build()
        .expect("reqwest client failed");

    let shared_client = Arc::new(http_client);
    dotenv().ok();
    let database_url = env::var("DB_CONNECTION_STRING").expect("DB_CONNECTION_STRING must be set");
    let postgres_client = PgPool::connect(&database_url).await.expect("connect to db failed");

    let (tx, rx) = mpsc::channel::<String>(10000);

    let discover_tx = tx.clone();
    drop(tx);
    let seed = "mastodon.social";
    discover_tx.send(seed.to_string()).await.expect("send failed");
    found_urls.insert(seed.to_string());

    let mut db_set = tokio::task::JoinSet::new();

    let mut stream = ReceiverStream::new(rx)
        .map(|url: String| {
            let client = shared_client.clone();
            let visited_clone = found_urls.clone();
            let tx_discovery = discover_tx.clone(); // Clone sender for the task

            async move {
                let fetch_future = fetch_instance(url.clone(), client);

                match tokio::time::timeout(Duration::from_secs(3), fetch_future).await {
                    Ok(Ok(result_tuple)) => {
                        let peers = result_tuple.1.clone();
                        tokio::spawn(async move {
                            for peer in peers {
                                if !peer.contains("troll") && visited_clone.insert(peer.clone()) {
                                    let _ = tx_discovery.send(peer).await;
                                }
                            }
                        });

                        (url, Ok(result_tuple))
                    }
                    Ok(Err(e)) => (url, Err(e)),
                    Err(_) => (url, Err(anyhow::anyhow!("Task timed out")))
                }
            }
        })
        .buffer_unordered(200);

    let mut index = 0;
    let mut total_attempts = 0;
    while let Ok(Some((url, result))) = timeout(Duration::from_secs(15), stream.next()).await {
        total_attempts += 1;

        match result {
            Ok(result_tuple) => {
                if let Some(node_info) = result_tuple.0 {
                    let pool_clone = postgres_client.clone();
                    let url_for_db = url.clone(); // Clone for the async move

                    db_set.spawn(async move {
                        save_data(url_for_db, node_info, &pool_clone).await;
                    });

                    index += 1;
                    if index % 10 == 0 {
                        println!("🚀 Success: {} | Queue: {} | Last: {}", index, total_attempts, url);
                    }
                } else {}
            }
            Err(_) => {
            }
        }
        if total_attempts % 100 == 0 {
            println!("📡 Progress: {} domains checked...", total_attempts);
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
