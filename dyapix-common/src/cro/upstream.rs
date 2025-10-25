use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{CRO, CRO_KIND_UPSTREAM};

/// A parsed upstream node with host and port
#[derive(Debug, Clone)]
pub struct UpstreamNode {
    pub host: String,
    pub port: u16,
    pub weight: u32,
}

/// Parse a node string (e.g., "host:80") into host and port
impl UpstreamNode {
    pub fn parse(address: &str) -> anyhow::Result<Self> {
        let parts: Vec<&str> = address.split(':').collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!("Invalid node address format: {}", address));
        }
        let host = parts[0].to_string();
        let port = parts[1]
            .parse()
            .map_err(|e| anyhow::anyhow!("Invalid port in address {}: {}", address, e))?;
        Ok(UpstreamNode {
            host,
            port,
            weight: 1, // Default weight
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Upstream {
    pub id: String,
    pub name: String,
    pub desc: Option<String>,
    #[serde(rename = "type")]
    pub type_: LoadBalanceType,
    pub scheme: Scheme,
    pub nodes: HashMap<String, u32>,
    pub retries: Option<u32>,
    pub timeout: Option<Timeout>,
}

impl Upstream {
    /// Parse all nodes from the HashMap and return a vector of UpstreamNode
    pub fn parse_nodes(&self) -> anyhow::Result<Vec<UpstreamNode>> {
        let mut parsed_nodes = Vec::new();
        for (address, weight) in &self.nodes {
            let mut node = UpstreamNode::parse(address)?;
            node.weight = *weight;
            parsed_nodes.push(node);
        }
        Ok(parsed_nodes)
    }
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

