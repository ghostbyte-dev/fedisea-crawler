use std::env;
use dotenvy::dotenv;
use redis::aio::ConnectionManager;
use crate::client::HttpClient;
use crate::consts::WORKERS;
use crate::db::RedisRepository;
use crate::postgres_db::PostgresRepository;

mod db;
mod worker;
mod client;
mod models;
mod postgres_db;
mod consts;

#[tokio::main]
async fn main() {
    dotenv().ok();

    let redis_url = env::var("REDIS_URL").expect("REDIS_URL must be set");
    println!("{}", redis_url);
    let redis_client = redis::Client::open(redis_url).expect("Failed to create redis client");
    let redis_manager = ConnectionManager::new(redis_client).await.expect("Failed to create redis manager");
    let redis_repo = RedisRepository::new(redis_manager);

    let pg_url = env::var("DB_CONNECTION_STRING").expect("DATABASE_URL must be set");
    let pg_pool = sqlx::PgPool::connect(&*pg_url).await.expect("Failed to connect to postgres pool");
    let pg_repo = PostgresRepository::new(pg_pool);

    pg_repo.init()
        .await
        .expect("Failed to initialize database tables");

    let http_client = HttpClient::new();

  /* let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).expect("Time went backwards").as_secs() + 10;

   redis_repo.mark_as_seen("mastodon.social").await.expect("Failed to mark mastodon");
    redis_repo.enqueue_job("mastodon.social", now as i64).await.expect("Failed to enqueue mastodon");*/
/*
    redis_repo.mark_as_seen("pixelfed.social").await.expect("Failed to mark mastodon");
    redis_repo.enqueue_job("pixelfed.social", now as i64).await.expect("Failed to enqueue mastodon");
*/

   for i in 0..WORKERS {
        let r_repo = redis_repo.clone();
        let p_repo = pg_repo.clone();
        let h_client = http_client.clone();

        tokio::spawn(async move {
            println!("Worker #{} is online", i);
            worker::run_worker(r_repo, p_repo, h_client).await;
        });
    }
    
    tokio::signal::ctrl_c().await.expect("failed to listen for event");
    println!("Shutting down...");
}