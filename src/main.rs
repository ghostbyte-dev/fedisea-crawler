use crate::client::HttpClient;
use crate::consts::WORKERS;
use crate::db::RedisRepository;
use crate::location_lookup::{lookup_asn_organisation, lookup_country, lookup_ip};
use crate::postgres_db::PostgresRepository;
use dotenvy::dotenv;
use hickory_resolver::Resolver;
use hickory_resolver::config::ResolverConfig;
use hickory_resolver::name_server::TokioConnectionProvider;
use maxminddb::Reader;
use redis::aio::ConnectionManager;
use sqlx::postgres::PgPoolOptions;
use std::env;
use std::sync::Arc;

extern crate jemallocator;

mod client;
mod consts;
mod db;
pub mod domain_filter;
pub mod location_lookup;
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

    let domain_resolver = Resolver::builder_with_config(
        ResolverConfig::cloudflare(),
        TokioConnectionProvider::default(),
    )
    .build();

    let asn_reader = Arc::new(unsafe {
        Reader::open_mmap(env::var("MAXMIND_DB_PATH_ASN").expect("MAXMIND_DB_PATH_ASN"))
            .expect("Failed to open reader")
    });
    let country_reader = Arc::new(unsafe {
        Reader::open_mmap(env::var("MAXMIND_DB_PATH_COUNTRY").expect("MAXMIND_DB_PATH_COUNTRY"))
            .expect("Failed to open reader")
    });
    let city_reader = Arc::new(unsafe {
        Reader::open_mmap(env::var("MAXMIND_DB_PATH_CITY").expect("MAXMIND_DB_PATH_CITY"))
            .expect("Failed to open reader")
    });

    let pg_repo = PostgresRepository::new(pg_pool);

    let http_client = HttpClient::new();

    // add seed in the first run
    //add_seed(redis_repo.clone()).await;
    for i in 0..WORKERS {
        let r_repo = redis_repo.clone();
        let p_repo = pg_repo.clone();
        let h_client = http_client.clone();
        let domain_resolver_client = domain_resolver.clone();
        let asn_reader_client = asn_reader.clone();
        let country_reader_client = country_reader.clone();
        let city_reader_client = city_reader.clone();

        tokio::spawn(async move {
            println!("Worker #{} is online", i);
            worker::run_worker(
                r_repo,
                p_repo,
                h_client,
                &domain_resolver_client,
                &asn_reader_client,
                &country_reader_client,
                &city_reader_client
            )
            .await;
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
