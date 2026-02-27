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
    ROBOTTXT,
    MISMATCHED
}

impl InstanceStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ACTIVE => "ACTIVE",
            Self::DEAD => "DEAD",
            Self::DOWN => "DOWN",
            Self::ROBOTTXT => "ROBOTTXT",
            Self::MISMATCHED => "MISMATCHED"
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

    #[error("Mismatched url, redirects or nodeinfo url doesnt match")]
    Mismatched(String)
}

pub struct InstanceInfo {
    pub title: Option<String>,
    pub description: Option<String>,
    pub email: Option<String>,
    pub thumbnail: Option<String>,
    pub source_url: Option<String>,
    pub icon: Option<String>,
}

#[derive(Deserialize)]
pub struct MastodonV2Response {
    title: Option<String>,
    description: Option<String>,
    contact: ContactMastodon,
    source_url: Option<String>,
    thumbnail: ThumbnailMastodon,
}

#[derive(Deserialize)]
struct ContactMastodon {
    pub email: Option<String>
}

#[derive(Deserialize)]
struct ThumbnailMastodon {
    pub url: Option<String>
}

impl From<MastodonV2Response> for InstanceInfo {
    fn from(m: MastodonV2Response) -> Self {
        Self {
            title: m.title,
            description: m.description,
            email: m.contact.email,
            thumbnail: m.thumbnail.url,
            source_url: m.source_url,
            icon: None
        }
    }
}

#[derive(Deserialize)]
pub struct LemmyInfoResponse {
    site_view: LemmySiteView
}


#[derive(Deserialize)]
pub struct LemmySiteView {
    pub site: LemmySite,
}

#[derive(Deserialize)]
pub struct LemmySite {
    pub name: Option<String>,
    pub description: Option<String>,
}

impl From<LemmyInfoResponse> for InstanceInfo {
    fn from(m: LemmyInfoResponse) -> Self {
        Self {
            title: m.site_view.site.name,
            description: m.site_view.site.description,
            email: None,
            thumbnail: None,
            source_url: None,
            icon: None,
        }
    }
}

#[derive(Deserialize)]
pub struct PeertubeInfoResponse {
    instance: PeertubeInstance
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PeertubeInstance {
    pub name: Option<String>,
    pub description: Option<String>,
}

impl From<PeertubeInfoResponse> for InstanceInfo {
    fn from(m: PeertubeInfoResponse) -> Self {
        Self {
            title: m.instance.name,
            description: m.instance.description,
            email: None,
            thumbnail: None,
            source_url: None,
            icon: None
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MisskeyInfoResponse {
    name: Option<String>,
    description: Option<String>,
    repository_url: Option<String>,
    background_image_url: Option<String>,
    icon_url: Option<String>,
}

impl From<MisskeyInfoResponse> for InstanceInfo {
    fn from(m: MisskeyInfoResponse) -> Self {
        Self {
            title: m.name,
            description: m.description,
            email: None,
            thumbnail: m.background_image_url,
            source_url: m.repository_url,
            icon: m.icon_url
        }
    }
}