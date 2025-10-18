use serde::{Deserialize, Serialize};

use super::{CRO, CRO_KIND_TLS_CERT};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsCert {
    pub id: String,
    pub cert: String,
    pub key: String,
    pub snis: Vec<String>,
}

impl CRO for TlsCert {
    fn cro_kind() -> &'static str {
        CRO_KIND_TLS_CERT
    }

    fn id(&self) -> &str {
        &self.id
    }
}

