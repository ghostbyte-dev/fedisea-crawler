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
pub struct Nodeinfo {
    pub software: Software,
}

#[derive(Deserialize)]
pub struct Software {
    pub name: String,
    pub version: String,
}
