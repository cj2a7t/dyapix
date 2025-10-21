use async_trait::async_trait;
use dyapix_common::{cache::routes_cache, cro::{Route, Upstream}};
use pingora::{
    prelude::HttpPeer,
    prelude::*,
    proxy::{ProxyHttp, Session},
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
