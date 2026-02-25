use std::sync::Arc;
use std::time::Duration;
use reqwest::{Client, Url};
use crate::models::{Nodeinfo, WellKnown};

#[derive(Clone)]
pub struct HttpClient {
    http: Client,
}

impl HttpClient {
    pub fn new() -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(10))
            .user_agent("MyCrawler/1.0 (+https://github.com/your-username/crawler)")
            .build()
            .unwrap();

        Self { http }
    }

    pub async fn fetch_well_known(&self,
        instance: String,
    ) -> Result<WellKnown, anyhow::Error> {
        let url = format!("https://{}/.well-known/nodeinfo", instance, );
        let url = Url::parse(&*url)?;

        let response: WellKnown = self.http
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(response)
    }

    pub async fn fetch_peers(&self,
        instance: String) -> Result<Vec<String>, anyhow::Error> {
        let url = format!("https://{}/api/v1/instance/peers", instance);
        let url = Url::parse(&*url)?;

        let res: Vec<String> = self.http
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(res)
    }

    pub async fn fetch_nodeinfo(&self,
        url: &str
    ) -> Result<Nodeinfo, anyhow::Error> {
        let url = Url::parse(url)?;

        let res: Nodeinfo = self.http
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(res)
    }
}
