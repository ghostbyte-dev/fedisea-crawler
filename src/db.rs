use std::time::{SystemTime, UNIX_EPOCH};
use redis::aio::ConnectionManager;
use redis::AsyncCommands;

#[derive(Clone)]
pub struct RedisRepository {
    manager: ConnectionManager,
}

impl RedisRepository {
    pub fn new(manager: ConnectionManager) -> Self {
        Self { manager }
    }

    pub async fn mark_as_seen(&self, domain: &str) -> Result<bool, anyhow::Error> {
        let mut conn = self.manager.clone();

        let added: i32 = conn.sadd("crawler:seen_set", domain).await?;
        Ok(added == 1)
    }

    pub async fn enqueue_job(&self, domain: &str, run_at: i64) -> Result<(), anyhow::Error> {
        let mut conn = self.manager.clone();

        let _: i32 = conn.zadd("crawler:queue", domain, run_at).await?;
        Ok(())
    }

    pub async fn fetch_next_job(&self) -> anyhow::Result<Option<String>> {
        let mut conn = self.manager.clone();

        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;

        let jobs: Vec<String> = redis::cmd("ZRANGEBYSCORE")
            .arg("crawler:queue")
            .arg("-inf")
            .arg(now)
            .arg("LIMIT")
            .arg(0)
            .arg(1)
            .query_async(&mut conn)
            .await?;

        if let Some(domain) = jobs.into_iter().next() {
            let _: i32 = redis::cmd("ZREM")
                .arg("crawler:queue")
                .arg(&domain)
                .query_async(&mut conn)
                .await?;

            return Ok(Some(domain));
        }

        Ok(None)
    }
    
    pub async fn increment_failure(&self, domain: &str) -> i32 {
        let mut conn = self.manager.clone();
        conn.hincr(format!("stats:{}", domain), "fail_count", 1).await.unwrap_or(1)
    }

    pub async fn reset_failure(&self, domain: &str) {
        let mut conn = self.manager.clone();
        let _: () = conn.hset(format!("stats:{}", domain), "fail_count", 0).await.unwrap_or(());
    }
}
