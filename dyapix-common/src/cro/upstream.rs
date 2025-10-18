use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{CRO, CRO_KIND_UPSTREAM};

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

impl CRO for Upstream {
    fn cro_kind() -> &'static str {
        CRO_KIND_UPSTREAM
    }

    fn id(&self) -> &str {
        &self.id
    }
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

