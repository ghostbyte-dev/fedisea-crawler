use sqlx::PgPool;
use crate::models::Nodeinfo;

#[derive(Clone)]
pub struct PostgresRepository {
    pool: PgPool,
}

impl PostgresRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn save_data(&self, instance: String, nodeinfo: Nodeinfo) {
        let query = "
        INSERT INTO instance (
            domain, software, software_version, open_registration,
            total_users, active_users_month, active_users_halfyear,
            local_posts, local_comments
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        ON CONFLICT (domain)
        DO UPDATE SET
            software = EXCLUDED.software,
            software_version = EXCLUDED.software_version,
            open_registration = EXCLUDED.open_registration,
            total_users = EXCLUDED.total_users,
            active_users_month = EXCLUDED.active_users_month,
            active_users_halfyear = EXCLUDED.active_users_halfyear,
            local_posts = EXCLUDED.local_posts,
            local_comments = EXCLUDED.local_comments;
    ";

        // Note: Use &self.pool (reference) and ensure types match (i32/i64)
        let result = sqlx::query(query)
            .bind(instance.clone())
            .bind(nodeinfo.software.name)
            .bind(nodeinfo.software.version)
            .bind(nodeinfo.open_registrations)
            .bind(nodeinfo.usage.users.total)           // Explicit cast to match DB
            .bind(nodeinfo.usage.users.active_month)    // Explicit cast
            .bind(nodeinfo.usage.users.active_halfyear) // Explicit cast
            .bind(nodeinfo.usage.local_posts)           // Explicit cast
            .bind(nodeinfo.usage.local_comments)        // Explicit cast
            .execute(&self.pool) // Pass by reference!
            .await;

        if let Err(e) = result {
            eprintln!("❌ Database error for {}: {}", instance, e);
        }
    }
}
