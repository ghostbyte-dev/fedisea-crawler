use crate::client::HttpClient;
use crate::db::RedisRepository;
use crate::domain_filter::is_valid;
use crate::models::{CrawlerError, InstanceInfo, InstanceStatus, Nodeinfo, WellKnown};
use crate::postgres_db::PostgresRepository;
use reqwest::Url;
use futures::stream::{StreamExt, FuturesUnordered};
use tokio::time::interval;

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
                match process_instance(&instance, &http_client, &redis_repo).await {
                    Ok((instance, nodeinfo, instance_info, delay)) => {
                        redis_repo.reset_failure(&instance).await;
                        let db_result = postgres_repository
                            .save_data(instance.to_string(), nodeinfo, instance_info)
                            .await;
                        match db_result {
                            Ok(is_saved) => {
                                if is_saved {
                                    redis_repo.enqueue_job(&instance, now + delay).await.ok();
                                } else {
                                    println!("Skipping re-queue for {}: Instance is blocked.", instance);                                }
                            }
                            Err(e) => {
                                println!("Database error: {}", e)
                            }
                        }
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
                    Err(CrawlerError::Mismatched(points_to)) => {
                        println!("{}", points_to);
                        postgres_repository
                            .set_mismatched(&instance, &points_to)
                            .await;
                        redis_repo
                            .enqueue_job(&instance, now + 30 * 86400)
                            .await
                            .ok();
                        add_instance_to_queue(points_to, &redis_repo).await;
                    }
                    Err(_) => {
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

const NODEINFO_BASE_REL: &str = "http://nodeinfo.diaspora.software/ns/schema/";

pub fn find_latest_nodeinfo_url(well_known: &WellKnown) -> Result<(String, f32), anyhow::Error> {
    well_known
        .links
        .iter()
        .filter_map(|link| {
            let version_str = link.rel.strip_prefix(NODEINFO_BASE_REL)?;
            
            let version = version_str.parse::<f32>().ok()?;
            
            Some((version, link.href.trim().to_string()))
        })
        .max_by(|(v1, _), (v2, _)| v1.partial_cmp(v2).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(version, href)| (href, version))
        .ok_or_else(|| anyhow::anyhow!("Required NodeInfo rel format not found"))
}

pub async fn process_instance(
    instance: &str,
    http: &HttpClient,
    redis_repo: &RedisRepository,
) -> anyhow::Result<(String, Nodeinfo, Option<InstanceInfo>, i64), CrawlerError> {
    match http.are_robots_allowed(instance).await {
        Ok(true) => {}
        Ok(false) => return Err(CrawlerError::RobotsForbidden(instance.to_string())),
        Err(_) => return Err(CrawlerError::NetworkError("Failed to fetch".to_string())),
    }

    let well_known: (WellKnown, String) = http
        .fetch_well_known(instance.to_string())
        .await
        .map_err(|e| CrawlerError::NetworkError(e.to_string()))?;

    if well_known.1 != instance {
        return Err(CrawlerError::Mismatched(well_known.1));
    }
    
    let (url, version) = find_latest_nodeinfo_url(&well_known.0)
         .map_err(|_| CrawlerError::InvalidMetadata)?;

    let nodeinfo_url = Url::parse(&url.as_str())
        .map_err(|_| CrawlerError::InvalidMetadata)?;

    let info = http
        .fetch_nodeinfo(nodeinfo_url, version)
        .await
        .map_err(|e| CrawlerError::NetworkError(e.to_string()))?;

    let instance_info: Option<InstanceInfo> = match info.software.name.as_str() {
        "mastodon" | "pixelfed" | "pleroma" => {
            http.fetch_instance_info_mastodonish(&instance).await.ok()
        }
        "lemmy" => http.fetch_instance_info_lemmy(&instance).await.ok(),
        "peertube" => http.fetch_instance_info_peertube(&instance).await.ok(),
        "misskey" => http.fetch_instance_info_misskey(&instance).await.ok(),
        _ => None,
    };

    handle_peers(redis_repo, http, instance.parse().unwrap()).await;
    Ok((instance.parse().unwrap(), info, instance_info, 604800))
}

async fn handle_peers(redis_repo: &RedisRepository, http: &HttpClient, instance: String) {
    if let Ok(peers) = http.fetch_peers(instance).await {
        for chunk in peers.chunks(100) {
            let mut to_process = Vec::new();
            for peer in chunk {
                let p = peer.to_lowercase();
                if is_valid(&p) {
                    to_process.push(p);
                }
            }

            if !to_process.is_empty() {
                let _ = redis_repo.enqueue_batch_if_new(to_process).await;
            }

            tokio::task::yield_now().await;
        }
    }
}

async fn add_instance_to_queue(instance: String, redis_repo: &RedisRepository) {
    let peer = instance.to_lowercase();
    if is_valid(&peer) {
        if redis_repo.mark_as_seen(&peer).await.unwrap_or(false) {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("Time went backwards")
                .as_secs();

            let _ = redis_repo.enqueue_job(&peer, now as i64).await;
        }
    }
}