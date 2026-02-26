use crate::consts::USER_AGENT;
use crate::models::{Nodeinfo, WellKnown};
use reqwest::{Client, Url};
use robotstxt_rs::RobotsTxt;
use std::time::Duration;

#[derive(Clone)]
pub struct HttpClient {
    http: Client,
}

impl HttpClient {
    pub fn new() -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent(USER_AGENT)
            .build()
            .unwrap();

        Self { http }
    }

    pub async fn fetch_well_known(&self, instance: String) -> Result<(WellKnown, String), anyhow::Error> {
        let url = format!("https://{}/.well-known/nodeinfo", instance,);
        let url = Url::parse(&*url)?;

        let resp = self.http
            .get(url)
            .send()
            .await?
            .error_for_status()?;

        let final_url = resp.url().clone();
        let final_host = final_url.host_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid host in final URL"))?
            .to_string();

        let node_info_links: WellKnown = resp.json().await?;

        Ok((node_info_links, final_host))
    }

    pub async fn fetch_peers(&self, instance: String) -> Result<Vec<String>, anyhow::Error> {
        let url = format!("https://{}/api/v1/instance/peers", instance);
        let url = Url::parse(&*url)?;

        let res: Vec<String> = self
            .http
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(res)
    }

    pub async fn fetch_nodeinfo(&self, url: &str) -> Result<Nodeinfo, anyhow::Error> {
        let url = Url::parse(url)?;

        let res: Nodeinfo = self
            .http
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(res)
    }
    pub async fn are_robots_allowed(&self, instance: &str) -> Result<bool, anyhow::Error> {
        let Ok(domain) = Url::parse(&format!("https://{}", instance)) else {
            return Err(anyhow::anyhow!("Invalid Instance url {}", instance));
        };

        let robots_url = format!(
            "{}://{}/robots.txt",
            domain.scheme(),
            domain.host_str().unwrap_or("")
        );

        let response = self.http.get(&robots_url).send().await;

        let body = match response {
            Ok(res) if res.status().is_success() => res.text().await.unwrap_or_default(),
            Ok(res) if res.status() == reqwest::StatusCode::NOT_FOUND => {
                return Ok(true);
            }
            _ => return Err(anyhow::anyhow!("Failed to get robots.txt response")),
        };

        let robots = RobotsTxt::parse(&body);
        let target_path = "/.well-known/nodeinfo";
        Ok(robots.can_fetch(
            "Fedisea (https://github.com/ghostbyte-dev/fedisea-crawler)",
            target_path,
        ))
    }
}
