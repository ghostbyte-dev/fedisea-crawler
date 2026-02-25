use serde::Deserialize;

#[derive(Deserialize)]
pub struct WellKnown {
    pub links: Vec<WellKnownElement>,
}

#[derive(Deserialize)]
pub struct WellKnownElement {
    #[warn(dead_code)]
    pub rel: String,
    pub href: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Nodeinfo {
    pub software: Software,
    pub open_registrations: bool,
    pub usage: Usage
}

#[derive(Deserialize)]
pub struct Software {
    pub name: String,
    pub version: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Usage {
    pub local_posts: Option<i32>,
    pub local_comments: Option<i32>,
    pub users: UsersUsage
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsersUsage {
    pub total: Option<i32>,
    pub active_halfyear: Option<i32>,
    pub active_month: Option<i32>
}

#[derive(sqlx::Type)]
#[sqlx(type_name = "instance_status", rename_all = "lowercase")]
pub enum InstanceStatus {
    ACTIVE,
    DEAD,
    DOWN,
    ERROR,
}

impl InstanceStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ACTIVE => "ACTIVE",
            Self::DEAD => "DEAD",
            Self::DOWN => "DOWN",
            Self::ERROR => "ERROR",
        }
    }
}