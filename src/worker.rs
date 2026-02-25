use crate::client::HttpClient;
use crate::db::RedisRepository;
use crate::postgres_db::PostgresRepository;

pub async fn run_worker(redis_repo: RedisRepository, postgres_repository: PostgresRepository, http_client: HttpClient) {
    println!("Running worker");
    loop {
        match redis_repo.fetch_next_job().await {
            Ok(Some(instance)) => {
                let mut next_run_delay = 86400;
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH).expect("Time went backwards").as_secs();

                match http_client.fetch_well_known(instance.clone()).await {
                    Ok(well_known) if !well_known.links.is_empty() => {
                        next_run_delay = 604800;
                        println!("Got instance: {}", well_known.links[0].href);
                        let nodeinfo = match http_client
                            .fetch_nodeinfo(well_known.links[0].href.trim())
                            .await
                        {
                            Ok(nodeinfo) => Some(nodeinfo),
                            Err(_) => {
                                next_run_delay = 86400;
                                None
                            }
                        };
                        if let Some(info) = nodeinfo {
                            postgres_repository.save_data(instance.clone(), info).await;
                        }

                        if let Ok(peers) = http_client.fetch_peers(instance.clone()).await {
                            for peer in peers {
                                if !peer.contains("troll") {
                                    if redis_repo.mark_as_seen(&peer).await.unwrap_or(false) {
                                        redis_repo.enqueue_job(&peer, now).await.ok();
                                    }
                                }
                            }
                        }
                    }
                    _ => {
                        println!("Failed to fetch instance {}", instance);
                    }
                };

                let run_at = now + next_run_delay;
                redis_repo
                    .enqueue_job(&instance, run_at)
                    .await
                    .expect("TODO: panic message");
            }
            Ok(None) => {
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
            Err(error) => {
                println!("Error: {}", error);
            }
        };
    }
}
