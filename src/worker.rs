use crate::client::HttpClient;
use crate::db::RedisRepository;
use crate::models::InstanceStatus;
use crate::postgres_db::PostgresRepository;

pub async fn run_worker(redis_repo: RedisRepository, postgres_repository: PostgresRepository, http_client: HttpClient) {
    println!("Running worker");
    loop {
        match redis_repo.fetch_next_job().await {
            Ok(Some(instance)) => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH).expect("Time went backwards").as_secs() as i64;

                match process_instance(&instance, &http_client, &postgres_repository, &redis_repo).await {
                    Ok(delay) => {
                        redis_repo.reset_failure(&instance).await;
                        redis_repo.enqueue_job(&instance, now + delay).await.ok();
                    }
                    Err(e) => {
                        let fail_count = redis_repo.increment_failure(&instance).await;

                        let days = (2_i64.pow(fail_count.saturating_sub(1) as u32)).min(30);
                        let delay = days * 86400;

                        redis_repo.enqueue_job(&instance, now + delay).await.ok();
                        if (fail_count <= 2) {
                            postgres_repository.update_status(&instance, InstanceStatus::DOWN).await;
                        } else {
                            postgres_repository.update_status(&instance, InstanceStatus::DEAD).await;
                        }
                    }
                }
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

async fn process_instance(
    instance: &str,
    http: &HttpClient,
    pg_repo: &PostgresRepository,
    redis_repo: &RedisRepository,
) -> anyhow::Result<i64> {
    let well_known = http.fetch_well_known(instance.to_string()).await?;
    let nodeinfo_url = well_known.links.first()
        .ok_or_else(|| anyhow::anyhow!("No links found"))?
        .href.trim();

    let info = http.fetch_nodeinfo(nodeinfo_url).await?;
    pg_repo.save_data(instance.to_string(), info).await;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).expect("Time went backwards").as_secs();

    if let Ok(peers) = http.fetch_peers(instance.to_string()).await {
        for peer in peers {
            if !peer.contains("troll") {
                if redis_repo.mark_as_seen(&peer).await.unwrap_or(false) {
                    let _ = redis_repo.enqueue_job(&peer, now as i64).await;
                }
            }
        }
    }

    Ok(604800)
}
