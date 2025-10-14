use router_radix::{RadixHttpMethod, RadixNode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Route {
    pub id: String,
    pub name: String,
    pub desc: Option<String>,
    pub priority: u32,
    pub label: Option<HashMap<String, String>>,
    pub uris: Vec<String>,
    pub methods: Option<Vec<String>>,
    pub hosts: Option<Vec<String>>,
    pub plugins: Option<HashMap<String, serde_json::Value>>,
    pub upstream_id: Option<String>,
    pub upstream: Option<Upstream>,
}

impl From<&Route> for RadixNode {
    fn from(route: &Route) -> Self {
        let methods = route.methods.as_ref().map(|m| {
            let mut result = RadixHttpMethod::empty();
            for method in m {
                if let Some(http_method) = RadixHttpMethod::from_str(method) {
                    result |= http_method;
                }
            }
            result
        });

        RadixNode {
            id: route.id.clone(),
            paths: route.uris.clone(),
            methods,
            hosts: route.hosts.clone(),
            remote_addrs: None,
            vars: None,
            filter_fn: None,
            priority: route.priority as i32,
            metadata: serde_json::to_value(route).unwrap_or(serde_json::Value::Null),
        }
    }
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
