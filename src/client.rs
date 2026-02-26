use crate::consts::USER_AGENT;
use crate::models::{
    InstanceInfo, LemmyInfoResponse, MastodonV2Response, MisskeyInfoResponse, Nodeinfo,
    PeertubeInfoResponse, WellKnown,
};
use reqwest::{Client, Url};
use robotxt::Robots;
use std::time::Duration;
use serde_json::json;

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

    pub async fn fetch_well_known(
        &self,
        instance: String,
    ) -> Result<(WellKnown, String), anyhow::Error> {
        let url = format!("https://{}/.well-known/nodeinfo", instance,);
        let url = Url::parse(&*url)?;

        let resp = self.http.get(url).send().await?.error_for_status()?;

        let final_url = resp.url().clone();
        let final_host = final_url
            .host_str()
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

        let target_path = "/.well-known/nodeinfo";
        let r = Robots::from_bytes(body.as_bytes(), USER_AGENT);

        Ok(r.is_relative_allowed(target_path))
    }

    pub async fn fetch_instance_info_mastodonish(
        &self,
        instance: &str,
    ) -> Result<InstanceInfo, anyhow::Error> {
        let url = format!("https://{}/api/v2/instance", instance);
        let url = Url::parse(&url)?;

        let res: MastodonV2Response = self
            .http
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(InstanceInfo::from(res))
    }

    pub async fn fetch_instance_info_lemmy(
        &self,
        instance: &str,
    ) -> Result<InstanceInfo, anyhow::Error> {
        let url = format!("https://{}/api/v3/site", instance);
        let url = Url::parse(&url)?;

        let res: LemmyInfoResponse = self
            .http
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(InstanceInfo::from(res))
    }

    pub async fn fetch_instance_info_peertube(
        &self,
        instance: &str,
    ) -> Result<InstanceInfo, anyhow::Error> {
        let url = format!("https://{}/api/v1/config/about", instance);
        let url = Url::parse(&url)?;

        let res: PeertubeInfoResponse = self
            .http
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(InstanceInfo::from(res))
    }

    pub async fn fetch_instance_info_misskey(
        &self,
        instance: &str,
    ) -> Result<InstanceInfo, anyhow::Error> {
        println!("misskey");
        let url = format!("https://{}/api/meta", instance);
        let url = Url::parse(&url)?;

        let res: MisskeyInfoResponse = self
            .http
            .post(url)
            .json(&json!({ "detail": false })) // Add the body here
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        println!("success");
        Ok(InstanceInfo::from(res))
    }
}
