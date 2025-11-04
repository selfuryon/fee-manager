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
pub struct DbProposerConfig {
    pub proposer: String,
    pub default_configs: Option<Vec<String>>,
    pub fee_recipient: Option<String>,
    pub gas_limit: Option<String>,
    pub min_value: Option<String>,
    pub grace: Option<i16>,
    pub reset_relays: Option<bool>,
    pub relays: Option<HashMap<String, DbRelayConfig>>,
}
