use crate::client::HttpClient;
use crate::db::RedisRepository;
use crate::domain_filter::is_valid;
use crate::models::{CrawlerError, InstanceInfo, InstanceStatus, Nodeinfo, WellKnown};
use crate::postgres_db::PostgresRepository;
use reqwest::Url;
use futures::stream::{StreamExt, FuturesUnordered};

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
                        redis_repo.enqueue_job(&instance, now + delay).await.ok();
                        postgres_repository
                            .save_data(instance.to_string(), nodeinfo, instance_info)
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

    let normalize = |s: &str| {
        s.strip_prefix("www.")
            .unwrap_or(s)
            .to_string()
    };

    let normalized_well_known = normalize(&well_known.1);
    let normalized_instance = normalize(&instance.to_string());

    if normalized_well_known != normalized_instance {
        return Err(CrawlerError::Mismatched(well_known.1));
    }

    let nodeinfo_url = well_known
        .0
        .links
        .first()
        .ok_or(CrawlerError::InvalidMetadata)?
        .href
        .trim();

    let nodeinfo_url = Url::parse(nodeinfo_url)
        .map_err(|_| CrawlerError::InvalidMetadata)?;
    let nodeinfo_url_domain_normalized = normalize(nodeinfo_url.domain().unwrap());

    if nodeinfo_url_domain_normalized != normalized_instance {
        return Err(CrawlerError::Mismatched(nodeinfo_url_domain_normalized.parse().unwrap()));
    }

    let info = http
        .fetch_nodeinfo(nodeinfo_url.as_ref())
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