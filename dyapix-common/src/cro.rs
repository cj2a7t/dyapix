use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Route {
    pub id: String,
    pub name: String,
    pub desc: Option<String>,
    pub priority: u32,
    pub label: Option<HashMap<String, String>>,
    pub uris: Vec<String>,
    pub methods: Vec<String>,
    pub hosts: Vec<String>,
    pub plugins: Option<HashMap<String, serde_json::Value>>,
    #[serde(flatten)]
    pub upstream_config: UpstreamConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UpstreamConfig {
    Reference { upstream_id: String },
    Inline { upstream: Upstream },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Upstream {
    pub id: String,
    pub name: String,
    pub desc: Option<String>,
    pub type_: LoadBalanceType,
    pub scheme: Scheme,
    pub nodes: HashMap<String, u32>,
    pub retries: Option<u32>,
    pub timeout: Option<Timeout>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LoadBalanceType {
    RoundRobin,
    LeastConn,
    ConsistentHash,
    WeightedRoundRobin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scheme {
    Http,
    Https,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timeout {
    pub connect: u32,
    pub send: u32,
    pub read: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsCert {
    pub id: String,
    pub cert: String,
    pub key: String,
    pub snis: Vec<String>,
}
