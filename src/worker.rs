use crate::client::HttpClient;
use crate::db::RedisRepository;
use crate::domain_filter::is_valid;
use crate::models::{CrawlerError, InstanceStatus, Nodeinfo, WellKnown};
use crate::postgres_db::PostgresRepository;
use anyhow::anyhow;

pub async fn run_worker(
    redis_repo: RedisRepository,
    postgres_repository: PostgresRepository,
    http_client: HttpClient,
) {
    println!("Running worker");
    loop {
        match redis_repo.fetch_next_job().await {
            Ok(Some(instance)) => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("Time went backwards")
                    .as_secs() as i64;
                match process_instance(&instance, &http_client, &redis_repo)
                    .await
                {
                    Ok((instance, nodeinfo, delay)) => {
                        println!("success: {}", instance);
                        redis_repo.reset_failure(&instance).await;
                        redis_repo.enqueue_job(&instance, now + delay).await.ok();
                        postgres_repository
                            .save_data(instance.to_string(), nodeinfo)
                            .await;
                    }
                    Err(CrawlerError::RobotsForbidden(instance)) => {
                        postgres_repository
                            .update_status(&instance, InstanceStatus::ROBOTTXT)
                            .await;
                        redis_repo
                            .enqueue_job(&instance, now + 30 * 86400)
                            .await
                            .ok();
                    }
                    Err(_) => {
                        println!("Failed to process instance: {}", instance);
                        let fail_count = redis_repo.increment_failure(&instance).await;

                        let days = (2_i64.pow(fail_count.saturating_sub(1) as u32)).min(30);
                        let delay = days * 86400;

                        redis_repo.enqueue_job(&instance, now + delay).await.ok();
                        if fail_count <= 2 {
                            postgres_repository
                                .update_status(&instance, InstanceStatus::DOWN)
                                .await;
                        } else {
                            postgres_repository
                                .update_status(&instance, InstanceStatus::DEAD)
                                .await;
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

pub async fn process_instance(
    instance: &str,
    http: &HttpClient,
    redis_repo: &RedisRepository,
) -> anyhow::Result<(String, Nodeinfo, i64), CrawlerError> {
    match http.are_robots_allowed(instance).await {
        Ok(true) => {},
        Ok(false) => return Err(CrawlerError::RobotsForbidden(instance.to_string())),
        Err(e) => return Err(CrawlerError::NetworkError("Failed to fetch".to_string())),
    }

    let well_known: (WellKnown, String) = http
        .fetch_well_known(instance.to_string())
        .await
        .map_err(|e| CrawlerError::NetworkError(e.to_string()))?;

    let instance = well_known.1;
    let nodeinfo_url = well_known
        .0
        .links
        .first()
        .ok_or(CrawlerError::InvalidMetadata)?
        .href
        .trim();

    let info = http
        .fetch_nodeinfo(nodeinfo_url)
        .await
        .map_err(|e| CrawlerError::NetworkError(e.to_string()))?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs();

    if let Ok(peers) = http.fetch_peers(instance.to_string()).await {
        for peer in peers {
            let peer = peer.to_lowercase();
            if is_valid(&peer) {
                if redis_repo.mark_as_seen(&peer).await.unwrap_or(false) {
                    let _ = redis_repo.enqueue_job(&peer, now as i64).await;
                }
            }
        }
    }

    Ok((instance, info, 604800))
}
