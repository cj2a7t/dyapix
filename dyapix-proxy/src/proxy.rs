use async_trait::async_trait;
use dyapix_common::{
    cache::{routes_cache, upstreams_cache},
    cro::{Route, Upstream},
};
use pingora::{
    Result,
    http::StatusCode,
    prelude::*,
    proxy::{ProxyHttp, Session},
};
use router_radix::RadixMatchOpts;

use crate::error::{
    AnyhowResultExt, ERROR_CACHE_NOT_INITIALIZED, ERROR_ROUTE_NOT_FOUND, ERROR_UPSTREAM_NOT_FOUND,
};
use crate::load_balancer::{DynamicLoadBalancer, build_load_balancer, get_sni_from_backend};

pub struct DyapixProxy;

pub struct DyapixProxyContext {
    pub matched_route: Option<Route>,
    pub matched_upstream: Option<Upstream>,
    pub load_balancer: Option<DynamicLoadBalancer>,
}

#[async_trait]
impl ProxyHttp for DyapixProxy {
    type CTX = DyapixProxyContext;

    fn new_ctx(&self) -> Self::CTX {
        DyapixProxyContext {
            matched_route: None,
            matched_upstream: None,
            load_balancer: None,
        }
    }

    async fn request_filter(&self, session: &mut Session, ctx: &mut Self::CTX) -> Result<bool> {
        let routes_cache = routes_cache::local().to_pingora_result(ERROR_CACHE_NOT_INITIALIZED)?;
        let upstreams_cache =
            upstreams_cache::local().to_pingora_result(ERROR_CACHE_NOT_INITIALIZED)?;

        let uri = session.req_header().uri.to_string();
        let method = session.req_header().method.to_string();
        let host = session
            .req_header()
            .headers
            .get("Host")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());

        let match_opts = RadixMatchOpts {
            method: Some(method),
            host: host,
            ..Default::default()
        };
        let match_result = routes_cache
            .routes_radix
            .match_route(uri.as_str(), &match_opts)
            .to_pingora_result(ERROR_ROUTE_NOT_FOUND)?
            .or_err(
                ErrorType::HTTPStatus(StatusCode::NOT_FOUND.into()),
                ERROR_ROUTE_NOT_FOUND,
            )?;
        let matched_route_id = match_result.id;

        let matched_route = routes_cache
            .routes_map
            .get(&matched_route_id)
            .ok_or(Error::create(
                ErrorType::HTTPStatus(StatusCode::NOT_FOUND.into()),
                ErrorSource::Downstream,
                Some(ImmutStr::from(ERROR_ROUTE_NOT_FOUND)),
                None,
            ))?;

        // Build load balancer from inline upstream or referenced upstream
        if let Some(upstream) = matched_route.upstream.clone() {
            ctx.matched_upstream = Some(upstream.clone());
            if let Ok(lb) = build_load_balancer(&upstream) {
                ctx.load_balancer = Some(lb);
            }
        } else if let Some(ref upstream_id) = matched_route.upstream_id {
            let upstream = upstreams_cache
                .upstreams_map
                .get(upstream_id)
                .ok_or(Error::create(
                    ErrorType::HTTPStatus(StatusCode::NOT_FOUND.into()),
                    ErrorSource::Downstream,
                    Some(ImmutStr::from(ERROR_UPSTREAM_NOT_FOUND)),
                    None,
                ))?
                .clone();
            ctx.matched_upstream = Some(upstream.clone());
            if let Ok(lb) = build_load_balancer(&upstream) {
                ctx.load_balancer = Some(lb);
            }
        }

        ctx.matched_route = Some(matched_route.clone());
        Ok(false)
    }

    async fn upstream_peer(
        &self,
        _session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>> {
        let (lb, upstream) = match (&ctx.load_balancer, &ctx.matched_upstream) {
            (Some(lb), Some(upstream)) => (lb, upstream),
            _ => {
                return Err(Error::create(
                    ErrorType::HTTPStatus(StatusCode::BAD_GATEWAY.into()),
                    ErrorSource::Upstream,
                    Some(ImmutStr::from("No upstream configured")),
                    None,
                ));
            }
        };

        // Select backend using configured load balancing strategy
        let Some(backend) = lb.select(b"", 32) else {
            return Err(Error::create(
                ErrorType::HTTPStatus(StatusCode::BAD_GATEWAY.into()),
                ErrorSource::Upstream,
                Some(ImmutStr::from("No healthy backend available")),
                None,
            ));
        };

        let tls = matches!(upstream.scheme, dyapix_common::cro::Scheme::Https);
        let host = get_sni_from_backend(&backend);
        let peer = HttpPeer::new(backend.clone(), tls, host);

        Ok(Box::new(peer))
    }
}
