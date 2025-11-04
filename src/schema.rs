// schema.rs
use crate::models::{DbDefaultConfig, DbProposerConfig, DbRelayConfig};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

#[derive(Clone, Serialize, Deserialize, ToSchema, Debug)]
pub struct ApiDefaultConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee_recipient: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_limit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grace: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relays: Option<HashMap<String, ApiRelayConfig>>,
}

#[derive(Clone, Serialize, Deserialize, ToSchema, Debug)]
pub struct ApiRelayConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee_recipient: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_limit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_value: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ApiProposerConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee_recipient: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_limit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grace: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reset_relays: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_configs: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relays: Option<HashMap<String, ApiRelayConfig>>,
}

impl From<DbDefaultConfig> for ApiDefaultConfig {
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

impl From<DbRelayConfig> for ApiRelayConfig {
    fn from(relay: DbRelayConfig) -> Self {
        Self {
            public_key: relay.public_key,
            fee_recipient: relay.fee_recipient,
            gas_limit: relay.gas_limit,
            min_value: relay.min_value,
        }
    }
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct SuccessResponse {
    pub message: String,
}

impl From<DbProposerConfig> for ApiProposerConfig {
    fn from(db: DbProposerConfig) -> Self {
        let relays = db
            .relays
            .map(|relays_map| relays_map.into_iter().map(|(k, v)| (k, v.into())).collect());

        Self {
            fee_recipient: db.fee_recipient,
            gas_limit: db.gas_limit,
            min_value: db.min_value,
            grace: db.grace,
            reset_relays: db.reset_relays,
            default_configs: db.default_configs,
            relays,
        }
    }
}
