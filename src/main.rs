use redis::aio::ConnectionManager;
use crate::client::HttpClient;
use crate::db::RedisRepository;
use crate::postgres_db::PostgresRepository;

mod db;
mod worker;
mod client;
mod models;
mod postgres_db;

#[tokio::main]
async fn main() {
    let redis_client = redis::Client::open("redis://127.0.0.1/").expect("Failed to create redis client");
    let redis_manager = ConnectionManager::new(redis_client).await.expect("Failed to create redis manager");
    let redis_repo = RedisRepository::new(redis_manager);

    let pg_url = "postgres://fedisea:Aut-1251@localhost/fedisea";
    let pg_pool = sqlx::PgPool::connect(pg_url).await.expect("Failed to connect to postgres pool");
    let pg_repo = PostgresRepository::new(pg_pool);

    let http_client = HttpClient::new();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).expect("Time went backwards").as_secs() + 10;

    redis_repo.mark_as_seen("mastodon.social").await.expect("Failed to mark mastodon");
    redis_repo.enqueue_job("mastodon.social", now).await.expect("Failed to enqueue mastodon");

    redis_repo.mark_as_seen("pixelfed.social").await.expect("Failed to mark mastodon");
    redis_repo.enqueue_job("pixelfed.social", now + 10).await.expect("Failed to enqueue mastodon");

    let repo_for_worker = redis_repo.clone();
    let postgres_for_worker = pg_repo.clone();
    let http_client_for_worker = http_client.clone();
    tokio::spawn(
        async move {
            worker::run_worker(repo_for_worker, postgres_for_worker, http_client_for_worker).await;
        }
    );

    tokio::signal::ctrl_c().await.expect("failed to listen for event");
    println!("Shutting down...");
}