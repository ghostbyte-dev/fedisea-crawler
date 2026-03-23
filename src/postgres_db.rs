use crate::models::{InstanceInfo, InstanceStatus, Nodeinfo};
use sqlx::PgPool;

#[derive(Clone)]
pub struct PostgresRepository {
    pool: PgPool,
}

impl PostgresRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn save_data(
        &self,
        instance: String,
        nodeinfo: Nodeinfo,
        instance_info: Option<InstanceInfo>,
    ) -> Result<bool, sqlx::Error> {

        // 1. Start a transaction
        let mut tx = self.pool.begin().await?;

        sqlx::query(
        "INSERT INTO software (identifier)
         VALUES ($1)
         ON CONFLICT (identifier) DO NOTHING"
    )
            .bind(&nodeinfo.software.name)
            .execute(&mut *tx)
            .await?;


        let query = "
        INSERT INTO instance (
            domain, software_id, software_version, open_registration,
            total_users, active_users_month, active_users_halfyear,
            local_posts, local_comments, status, title, description, email, thumbnail, source_url
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
        ON CONFLICT (domain)
        DO UPDATE SET
            software_id = EXCLUDED.software_id,
            software_version = EXCLUDED.software_version,
            open_registration = EXCLUDED.open_registration,
            total_users = EXCLUDED.total_users,
            active_users_month = EXCLUDED.active_users_month,
            active_users_halfyear = EXCLUDED.active_users_halfyear,
            local_posts = EXCLUDED.local_posts,
            local_comments = EXCLUDED.local_comments,
            status = EXCLUDED.status,
            title = EXCLUDED.title,
            description = EXCLUDED.description,
            email = EXCLUDED.email,
            thumbnail = EXCLUDED.thumbnail,
            source_url = EXCLUDED.source_url,
            points_to = NULL,
            last_seen = NOW()
        WHERE instance.status != 'BLOCKED';
    ";

        let result = sqlx::query(query)
            .bind(&instance)
            .bind(&nodeinfo.software.name)
            .bind(&nodeinfo.software.version)
            .bind(nodeinfo.open_registrations)
            .bind(nodeinfo.usage.users.total)
            .bind(nodeinfo.usage.users.active_month)
            .bind(nodeinfo.usage.users.active_halfyear)
            .bind(nodeinfo.usage.local_posts)
            .bind(nodeinfo.usage.local_comments)
            .bind(InstanceStatus::ACTIVE.as_str())
            .bind(instance_info.as_ref().map(|i| &i.title))
            .bind(instance_info.as_ref().and_then(|i| {
                i.description.as_ref().map(|desc| {
                    if desc.len() <= 255 {
                        desc.as_str()
                    } else {
                        match desc.char_indices().nth(255) {
                            Some((idx, _)) => &desc[..idx],
                            None => desc.as_str(),
                        }
                    }
                })
            }))
            .bind(instance_info.as_ref().map(|i| &i.email))
            .bind(instance_info.as_ref().map(|i| &i.thumbnail))
            .bind(instance_info.as_ref().map(|i| &i.source_url))
            .execute(&mut *tx)
            .await?;

        // 3. Handle Protocols only if the instance wasn't blocked
        if result.rows_affected() > 0 {
            // Remove old string-based associations
            sqlx::query("DELETE FROM instance_protocol WHERE instance_domain = $1")
                .bind(&instance)
                .execute(&mut *tx)
                .await?;

            for proto_name in nodeinfo.protocols {
                // Ensure protocol slug exists
                sqlx::query("INSERT INTO protocol (name) VALUES ($1) ON CONFLICT DO NOTHING")
                    .bind(&proto_name)
                    .execute(&mut *tx)
                    .await?;

                // Link Domain String to Protocol String
                sqlx::query(
                    "INSERT INTO instance_protocol (instance_domain, protocol_name) 
                     VALUES ($1, $2) ON CONFLICT DO NOTHING"
                )
                .bind(&instance)
                .bind(&proto_name)
                .execute(&mut *tx)
                .await?;
            }
            
            tx.commit().await?;
            Ok(true)
        } else {
            tx.rollback().await?;
            Ok(false)
        }
    }

    pub async fn update_status(&self, domain: &str, status: InstanceStatus) {
        let query = "
        INSERT INTO instance (domain, status, last_seen)
        VALUES ($1, $2, NOW())
        ON CONFLICT (domain)
        DO UPDATE SET
        status = EXCLUDED.status,
        last_seen = NOW()
        WHERE instance.status != 'BLOCKED';
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

    pub async fn set_mismatched(&self, domain: &str, points_to: &str) {
        let query = "
        INSERT INTO instance (domain, status, last_seen, points_to)
        VALUES ($1, $2, NOW(), $3)
        ON CONFLICT (domain)
        DO UPDATE SET
        status = EXCLUDED.status,
        points_to = EXCLUDED.points_to,
        last_seen = NOW();
        ";

        let result = sqlx::query(query)
            .bind(domain)
            .bind(InstanceStatus::MISMATCHED.as_str())
            .bind(points_to)
            .execute(&self.pool)
            .await;

        if let Err(e) = result {
            eprintln!("❌ Failed to update status for {}: {}", domain, e);
        }
    }
}
