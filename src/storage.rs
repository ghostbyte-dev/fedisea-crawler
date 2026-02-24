use crate::models::Nodeinfo;
use sqlx::PgPool;

pub async fn save_data(instance: String, nodeinfo: Nodeinfo, db_client: &PgPool) {
    println!("Saving data");
    let query = "
    INSERT INTO instance (domain, software, software_version, open_registration, total_users, active_users_month, active_users_halfyear, local_posts, local_comments)
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
    let result = sqlx::query(query)
        .bind(instance)
        .bind(nodeinfo.software.name)
        .bind(nodeinfo.software.version)
        .bind(nodeinfo.open_registrations)
        .bind(nodeinfo.usage.users.total)
        .bind(nodeinfo.usage.users.active_month)
        .bind(nodeinfo.usage.users.active_halfyear)
        .bind(nodeinfo.usage.local_posts)
        .bind(nodeinfo.usage.local_comments)
        .execute(db_client)
        .await;

    if let Err(e) = result {
        eprintln!("Database error: {}", e);
    }
}
