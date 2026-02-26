use serde::Deserialize;
use thiserror::Error;

#[derive(Deserialize)]
pub struct WellKnown {
    pub links: Vec<WellKnownElement>,
}

#[derive(Deserialize)]
pub struct WellKnownElement {
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

pub enum InstanceStatus {
    ACTIVE,
    DEAD,
    DOWN,
    ROBOTTXT
}

impl InstanceStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ACTIVE => "ACTIVE",
            Self::DEAD => "DEAD",
            Self::DOWN => "DOWN",
            Self::ROBOTTXT => "ROBOTTXT"
        }
    }
}

#[derive(Error, Debug)]
pub enum CrawlerError {
    #[error("Robots.txt forbids crawling for {0}")]
    RobotsForbidden(String),

    #[error("Network error or timeout: {0}")]
    NetworkError(String),

    #[error("Invalid NodeInfo format or missing links")]
    InvalidMetadata,
}