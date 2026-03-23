use crate::consts::USER_AGENT;
use crate::models::{
    InstanceInfo, LemmyInfoResponse, MastodonV2Response, MisskeyInfoResponse, Nodeinfo, NodeinfoV1, NodeinfoV2, PeertubeInfoResponse, WellKnown
};
use reqwest::{Client, Url};
use robotxt::Robots;
use serde_json::json;
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
            .pool_idle_timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(1)
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

    pub async fn fetch_nodeinfo(&self, url: Url, version: f32) -> Result<Nodeinfo, anyhow::Error> {
        match version {
            v if v == 1.0 || v == 1.1 => self.fetch_nodeinfo_v1(url).await,
            v if v >= 2.0 => self.fetch_nodeinfo_v2(url).await,
            _ => Err(anyhow::anyhow!("Unsupported NodeInfo version: {}", version)),
        }
    }

    pub async fn fetch_nodeinfo_v1(&self, url: Url) -> Result<Nodeinfo, anyhow::Error> {
        let res: NodeinfoV1 = self
            .http
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(res.into())
    }

    pub async fn fetch_nodeinfo_v2(&self, url: Url) -> Result<Nodeinfo, anyhow::Error> {
        let res: NodeinfoV2 = self
            .http
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(res.into())
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

        let response = self
            .http
            .get(&robots_url)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Network error: {}", e))?;

        let status = response.status();
        let body = if status.is_success() {
            response.text().await.unwrap_or_default()
        } else if status == reqwest::StatusCode::NOT_FOUND {
            return Ok(true);
        } else {
            return Ok(true);
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
