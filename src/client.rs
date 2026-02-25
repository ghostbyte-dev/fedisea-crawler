use std::sync::Arc;
use crate::models::{Nodeinfo, WellKnown};
use reqwest::{Client, Url};

pub async fn fetch_instance(
    instance: String,
    http_client: Arc<Client>,
) -> Result<(Option<Nodeinfo>, Vec<String>), anyhow::Error> {
    let well_known = fetch_well_known(instance.clone(), http_client.clone()).await?;

    let nodeinfo = if let Some(link) = well_known.links.first() {
        fetch_nodeinfo(&link.href, http_client.clone()).await.ok()
    } else {
        None
    };

    let peers = fetch_peers(instance, http_client).await.ok();
    let peers = peers.unwrap_or_else(|| vec![]);
    Ok((nodeinfo, peers))
}

pub async fn fetch_well_known(
    instance: String,
    http_client: Arc<Client>,
) -> Result<WellKnown, anyhow::Error> {
    let url = format!("https://{}/.well-known/nodeinfo", instance,);
    let url = Url::parse(&*url)?;

    let response = http_client
        .get(url.clone())
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Request Failed [{}] - Error: {}", url, e))?;

    let status = response.status();
    if !status.is_success() {
        return Err(anyhow::anyhow!(
            "HTTP Status Error [{}] - Status: {}",
            url,
            status
        ));
    }
    let body_text = response
        .text()
        .await
        .map_err(|e| anyhow::anyhow!("Body Read Error [{}] - Error: {}", url, e))?;

    let res: WellKnown = serde_json::from_str(&body_text).map_err(|e| {
        anyhow::anyhow!(
            "JSON Deserialization Error [{}] - Error: {}\nRaw Body (first 100 chars): {}",
            url,
            e,
            body_text.chars().take(100).collect::<String>()
        )
    })?;

    Ok(res)
}

pub async fn fetch_peers(
    instance: String,
    http_client: Arc<Client>,
) -> Result<Vec<String>, anyhow::Error> {
    let url = format!("https://{}/api/v1/instance/peers", instance);
    let url = Url::parse(&*url)?;

    let res: Vec<String> = http_client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(res)
}

pub async fn fetch_nodeinfo(
    url: &str,
    http_client: Arc<Client>,
) -> Result<Nodeinfo, anyhow::Error> {
    let url = Url::parse(url)?;

    let res: Nodeinfo = http_client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(res)
}
