use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub base_url: String,
    pub title: String,
    pub description: String,
    pub tagline: String,
    pub links: Links,
    pub playground: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Links {
    pub book: String,
    pub docs: String,
    pub api: String,
    pub github: String,
    pub discord: String,
}
