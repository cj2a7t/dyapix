use anyhow::Result;
use dyapix_common::cro::{LoadBalanceType, Upstream};
use pingora::lb::{Backend, LoadBalancer, selection::Consistent};

/// Trait for load balancer selection
pub trait LoadBalancerSelector: Send + Sync {
    fn select(&self, key: &[u8], max_iterations: usize) -> Option<Backend>;
}

/// Round Robin load balancer wrapper
struct RoundRobinSelector(LoadBalancer<pingora::lb::selection::RoundRobin>);

impl LoadBalancerSelector for RoundRobinSelector {
    fn select(&self, key: &[u8], max_iterations: usize) -> Option<Backend> {
        self.0.select(key, max_iterations)
    }
}

/// Consistent Hash load balancer wrapper
struct ConsistentHashSelector(LoadBalancer<Consistent>);

impl LoadBalancerSelector for ConsistentHashSelector {
    fn select(&self, key: &[u8], max_iterations: usize) -> Option<Backend> {
        self.0.select(key, max_iterations)
    }
}

/// Type alias for dynamic load balancer
pub type DynamicLoadBalancer = Box<dyn LoadBalancerSelector>;

/// Build a LoadBalancer from upstream nodes based on the load balance type
pub fn build_load_balancer(upstream: &Upstream) -> Result<DynamicLoadBalancer> {
    let nodes = upstream.parse_nodes()?;

    let mut backend_addresses = Vec::new();
    for node in nodes {
        backend_addresses.push(format!("{}:{}", node.host, node.port));
    }

    match upstream.type_ {
        LoadBalanceType::RoundRobin | LoadBalanceType::WeightedRoundRobin => Ok(Box::new(
            RoundRobinSelector(LoadBalancer::try_from_iter(backend_addresses)?),
        )),
        LoadBalanceType::ConsistentHash => Ok(Box::new(ConsistentHashSelector(
            LoadBalancer::try_from_iter(backend_addresses)?,
        ))),
        LoadBalanceType::LeastConn => {
            // LeastConn not yet supported in Pingora, fall back to RoundRobin
            Ok(Box::new(RoundRobinSelector(LoadBalancer::try_from_iter(
                backend_addresses,
            )?)))
        }
    }
}

/// Extract hostname from a backend for SNI
pub fn get_sni_from_backend(backend: &Backend) -> String {
    if let Some(inet_addr) = backend.addr.as_inet() {
        match inet_addr {
            std::net::SocketAddr::V4(v4) => v4.ip().to_string(),
            std::net::SocketAddr::V6(v6) => v6.ip().to_string(),
        }
    } else {
        format!("{}", backend.addr)
    }
}
