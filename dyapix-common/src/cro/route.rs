use router_radix::{RadixHttpMethod, RadixNode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{Upstream, CRO, CRO_KIND_ROUTE};

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

impl CRO for Route {
    fn cro_kind() -> &'static str {
        CRO_KIND_ROUTE
    }

    fn id(&self) -> &str {
        &self.id
    }
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

