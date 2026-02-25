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

    let res: WellKnown = http_client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
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
