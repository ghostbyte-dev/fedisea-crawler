use crate::client::HttpClient;
use crate::consts::WORKERS;
use crate::db::RedisRepository;
use crate::postgres_db::PostgresRepository;
use dotenvy::dotenv;
use redis::aio::ConnectionManager;
use std::env;
use sqlx::postgres::PgPoolOptions;

extern crate jemallocator;

mod client;
mod consts;
mod db;
pub mod domain_filter;
mod models;
mod postgres_db;
mod worker;

#[global_allocator]
static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;

#[tokio::main]
async fn main() {
    dotenv().ok();
    let redis_url = env::var("REDIS_URL").expect("REDIS_URL must be set");
    println!("{}", redis_url);
    let redis_client = redis::Client::open(redis_url).expect("Failed to create redis client");
    let redis_manager = ConnectionManager::new(redis_client)
        .await
        .expect("Failed to create redis manager");
    let redis_repo = RedisRepository::new(redis_manager);

    let pg_url = env::var("DB_CONNECTION_STRING").expect("DATABASE_URL must be set");
    let pg_pool = PgPoolOptions::new()
        .max_connections(5)
        .min_connections(1)
        .acquire_timeout(std::time::Duration::from_secs(3))
        .idle_timeout(std::time::Duration::from_secs(60))
        .connect(&pg_url)
        .await
        .expect("Failed to connect to postgres pool");
    let pg_repo = PostgresRepository::new(pg_pool);

    let http_client = HttpClient::new();

    // add seed in the first run
    //add_seed(redis_repo.clone()).await;

    for i in 0..WORKERS {
        let r_repo = redis_repo.clone();
        let p_repo = pg_repo.clone();
        let h_client = http_client.clone();

        tokio::spawn(async move {
            println!("Worker #{} is online", i);
            worker::run_worker(r_repo, p_repo, h_client).await;
        });
    }

    tokio::signal::ctrl_c()
        .await
        .expect("failed to listen for event");
    println!("Shutting down...");
}

async fn add_seed(redis_repository: RedisRepository) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs()
        + 10;
    redis_repository
        .mark_as_seen("mastodon.social")
        .await
        .expect("Failed to mark mastodon");
    redis_repository
        .enqueue_job("mastodon.social", now as i64)
        .await
        .expect("Failed to enqueue mastodon");
    redis_repository
        .mark_as_seen("pixelfed.social")
        .await
        .expect("Failed to mark mastodon");
    redis_repository
        .enqueue_job("pixelfed.social", now as i64)
        .await
        .expect("Failed to enqueue mastodon");
}
