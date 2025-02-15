// models.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Serialize, Deserialize)]
pub struct DbDefaultConfig {
    pub config_id: String,
    pub fee_recipient: Option<String>,
    pub gas_limit: Option<String>,
    pub min_value: Option<String>,
    pub grace: Option<i16>,
    pub relays: Option<HashMap<String, DbRelayConfig>>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DbRelayConfig {
    pub public_key: Option<String>,
    pub fee_recipient: Option<String>,
    pub gas_limit: Option<String>,
    pub min_value: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DefaultConfigResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee_recipient: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_limit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grace: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relays: Option<HashMap<String, RelayConfigResponse>>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RelayConfigResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee_recipient: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_limit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_value: Option<String>,
}

impl From<DbDefaultConfig> for DefaultConfigResponse {
    fn from(config: DbDefaultConfig) -> Self {
        Self {
            fee_recipient: config.fee_recipient,
            gas_limit: config.gas_limit,
            min_value: config.min_value,
            grace: config.grace,
            relays: config
                .relays
                .map(|relays| relays.into_iter().map(|(k, v)| (k, v.into())).collect()),
        }
    }
}

impl From<DbRelayConfig> for RelayConfigResponse {
    fn from(relay: DbRelayConfig) -> Self {
        Self {
            public_key: relay.public_key,
            fee_recipient: relay.fee_recipient,
            gas_limit: relay.gas_limit,
            min_value: relay.min_value,
        }
    }
}
