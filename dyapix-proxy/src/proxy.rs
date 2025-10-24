use async_trait::async_trait;
use dyapix_common::{
    cache::{routes_cache, upstreams_cache},
    cro::{Route, Upstream},
};
use pingora::{
    http::StatusCode,
    prelude::{HttpPeer, *},
    proxy::{ProxyHttp, Session},
};

use crate::error::{
    AnyhowResultExt, ERROR_CACHE_NOT_INITIALIZED, ERROR_ROUTE_NOT_FOUND, ERROR_UPSTREAM_NOT_FOUND,
};

pub struct DyapixProxy;

pub struct DyapixProxyContext {
    pub matched_route: Option<Route>,
    pub matched_upstream: Option<Upstream>,
}

#[async_trait]
impl ProxyHttp for DyapixProxy {
    type CTX = DyapixProxyContext;

    fn new_ctx(&self) -> Self::CTX {
        DyapixProxyContext {
            matched_route: None,
            matched_upstream: None,
        }
    }

    async fn request_filter(&self, session: &mut Session, ctx: &mut Self::CTX) -> Result<bool> {
        // Get the routes and upstreams cache
        let routes_cache = routes_cache::local().to_pingora_result(ERROR_CACHE_NOT_INITIALIZED)?;
        let upstreams_cache =
            upstreams_cache::local().to_pingora_result(ERROR_CACHE_NOT_INITIALIZED)?;

        // Use the radix router to match the route
        let match_result = routes_cache
            .routes_radix
            .match_route(
                session.req_header().uri.to_string().as_str(),
                &Default::default(),
            )
            .to_pingora_result(ERROR_ROUTE_NOT_FOUND)?
            .or_err(
                ErrorType::HTTPStatus(StatusCode::NOT_FOUND.into()),
                ERROR_ROUTE_NOT_FOUND,
            )?;
        let matched_route_id = match_result.id;

        // Get the matched route from the cache
        let matched_route = routes_cache
            .routes_map
            .get(&matched_route_id)
            .ok_or(Error::create(
                ErrorType::HTTPStatus(StatusCode::NOT_FOUND.into()),
                ErrorSource::Downstream,
                Some(ImmutStr::from(ERROR_ROUTE_NOT_FOUND)),
                None,
            ))?;

        if let Some(upstream) = matched_route.upstream.clone() {
            ctx.matched_upstream = Some(upstream);
        }
        // Use upstream id to get the upstream from the cache
        if let Some(ref upstream_id) = matched_route.upstream_id {
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
            ctx.matched_upstream = Some(upstream);
        }

        // Set the matched route to the context
        ctx.matched_route = Some(matched_route.clone());

        Ok(false)
    }

    async fn upstream_peer(
        &self,
        _session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>> {
        let mut peer = HttpPeer::new("127.0.0.1", false, "1.1.1.1".to_string());
        Ok(Box::new(peer))
    }
}
