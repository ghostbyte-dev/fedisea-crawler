use crate::models::{InstanceStatus, Nodeinfo};
use sqlx::PgPool;

#[derive(Clone)]
pub struct PostgresRepository {
    pool: PgPool,
}

impl PostgresRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn init(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS instance (
                domain TEXT PRIMARY KEY,
                software TEXT,
                software_version TEXT,
                open_registration BOOLEAN,
                total_users INTEGER,
                active_users_month INTEGER,
                active_users_halfyear INTEGER,
                local_posts INTEGER,
                local_comments INTEGER,
                status TEXT DEFAULT 'pending',
                last_seen TIMESTAMPTZ DEFAULT NOW()
            );
            "#
        )
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn save_data(&self, instance: String, nodeinfo: Nodeinfo) {
        let query = "
        INSERT INTO instance (
            domain, software, software_version, open_registration,
            total_users, active_users_month, active_users_halfyear,
            local_posts, local_comments, status
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        ON CONFLICT (domain)
        DO UPDATE SET
            software = EXCLUDED.software,
            software_version = EXCLUDED.software_version,
            open_registration = EXCLUDED.open_registration,
            total_users = EXCLUDED.total_users,
            active_users_month = EXCLUDED.active_users_month,
            active_users_halfyear = EXCLUDED.active_users_halfyear,
            local_posts = EXCLUDED.local_posts,
            local_comments = EXCLUDED.local_comments,
            status = EXCLUDED.status,
            last_seen = NOW();
    ";
    println!("{}", instance);
        // Note: Use &self.pool (reference) and ensure types match (i32/i64)
        let result = sqlx::query(query)
            .bind(instance.clone())
            .bind(nodeinfo.software.name)
            .bind(nodeinfo.software.version)
            .bind(nodeinfo.open_registrations)
            .bind(nodeinfo.usage.users.total)
            .bind(nodeinfo.usage.users.active_month)
            .bind(nodeinfo.usage.users.active_halfyear)
            .bind(nodeinfo.usage.local_posts)
            .bind(nodeinfo.usage.local_comments)
            .bind(InstanceStatus::ACTIVE.as_str())
            .execute(&self.pool)
            .await;

        if let Err(e) = result {
            eprintln!("❌ Database error for {}: {}", instance, e);
        }
    }

    pub async fn update_status(&self, domain: &str, status: InstanceStatus) {

        let query = "
        INSERT INTO instance (domain, status, last_seen)
        VALUES ($1, $2, NOW())
        ON CONFLICT (domain)
        DO UPDATE SET
        status = EXCLUDED.status,
        last_seen = NOW();
        ";

        let result = sqlx::query(query)
            .bind(domain)
            .bind(status.as_str())
            .execute(&self.pool)
            .await;

        if let Err(e) = result {
            eprintln!("❌ Failed to update status for {}: {}", domain, e);
        }
    }
}
