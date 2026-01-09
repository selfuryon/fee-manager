// schema.rs - API request/response types
use crate::addresses::{BlsPubkey, EthAddress};
use crate::models::{
    VouchDefaultConfig, VouchDefaultRelay, VouchProposer, VouchProposerPattern,
    VouchProposerPatternRelay, VouchProposerRelay,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

// ============================================================================
// Common Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RelayConfig {
    pub public_key: BlsPubkey,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee_recipient: Option<EthAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_limit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProposerRelayConfig {
    pub public_key: BlsPubkey,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee_recipient: Option<EthAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_limit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_value: Option<String>,
    #[serde(default)]
    pub disabled: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}


// ============================================================================
// Vouch - Default Configs API
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DefaultConfigResponse {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee_recipient: Option<EthAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_limit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_value: Option<String>,
    pub active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relays: Option<HashMap<String, RelayConfig>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DefaultConfigListItem {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee_recipient: Option<EthAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_limit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_value: Option<String>,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateDefaultConfigRequest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee_recipient: Option<EthAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_limit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_value: Option<String>,
    #[serde(default = "default_true")]
    pub active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relays: Option<HashMap<String, RelayConfig>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateDefaultConfigRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee_recipient: Option<EthAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_limit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relays: Option<HashMap<String, RelayConfig>>,
}

fn default_true() -> bool {
    true
}

// ============================================================================
// Vouch - Proposers API
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProposerResponse {
    pub public_key: BlsPubkey,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee_recipient: Option<EthAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_limit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_value: Option<String>,
    pub reset_relays: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relays: Option<HashMap<String, ProposerRelayConfig>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProposerListItem {
    pub public_key: BlsPubkey,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee_recipient: Option<EthAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_limit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_value: Option<String>,
    pub reset_relays: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateOrUpdateProposerRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee_recipient: Option<EthAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_limit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_value: Option<String>,
    #[serde(default)]
    pub reset_relays: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relays: Option<HashMap<String, ProposerRelayConfig>>,
}

// ============================================================================
// Vouch - Proposer Patterns API
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProposerPatternResponse {
    pub name: String,
    pub pattern: String,
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee_recipient: Option<EthAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_limit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_value: Option<String>,
    pub reset_relays: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relays: Option<HashMap<String, RelayConfig>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProposerPatternListItem {
    pub name: String,
    pub pattern: String,
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee_recipient: Option<EthAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_limit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_value: Option<String>,
    pub reset_relays: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateProposerPatternRequest {
    pub name: String,
    pub pattern: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee_recipient: Option<EthAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_limit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_value: Option<String>,
    #[serde(default)]
    pub reset_relays: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relays: Option<HashMap<String, RelayConfig>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateProposerPatternRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee_recipient: Option<EthAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_limit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reset_relays: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relays: Option<HashMap<String, RelayConfig>>,
}

// ============================================================================
// Vouch - Execution Config (Public Endpoint)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ExecutionConfigRequest {
    pub keys: Vec<BlsPubkey>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ExecutionConfigResponse {
    pub version: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee_recipient: Option<EthAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_limit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relays: Option<HashMap<String, RelayConfig>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proposers: Option<Vec<ProposerEntry>>,
}

/// Entry in proposers array - can be either a specific validator key or a regex pattern
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProposerEntry {
    /// Validator public key (BlsPubkey) or regex pattern (String)
    pub proposer: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee_recipient: Option<EthAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_limit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reset_relays: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relays: Option<HashMap<String, RelayConfig>>,
}

// ============================================================================
// Commit-Boost - Mux API
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MuxConfigResponse {
    pub name: String,
    pub keys: Vec<BlsPubkey>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MuxConfigListItem {
    pub name: String,
    pub key_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateMuxConfigRequest {
    pub name: String,
    #[serde(default)]
    pub keys: Vec<BlsPubkey>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateMuxConfigRequest {
    #[serde(default)]
    pub keys: Vec<BlsPubkey>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MuxKeysRequest {
    pub keys: Vec<BlsPubkey>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MuxKeysResponse {
    pub added: Option<i64>,
    pub removed: Option<i64>,
    pub total_keys: i64,
}

// ============================================================================
// Conversions
// ============================================================================

impl From<VouchDefaultConfig> for DefaultConfigListItem {
    fn from(config: VouchDefaultConfig) -> Self {
        Self {
            name: config.name,
            fee_recipient: config.fee_recipient,
            gas_limit: config.gas_limit,
            min_value: config.min_value,
            active: config.active,
            created_at: config.created_at,
            updated_at: config.updated_at,
        }
    }
}

impl From<VouchProposer> for ProposerListItem {
    fn from(proposer: VouchProposer) -> Self {
        Self {
            public_key: proposer.public_key,
            fee_recipient: proposer.fee_recipient,
            gas_limit: proposer.gas_limit,
            min_value: proposer.min_value,
            reset_relays: proposer.reset_relays,
            created_at: proposer.created_at,
            updated_at: proposer.updated_at,
        }
    }
}

impl From<VouchProposerPattern> for ProposerPatternListItem {
    fn from(pattern: VouchProposerPattern) -> Self {
        Self {
            name: pattern.name,
            pattern: pattern.pattern,
            tags: pattern.tags,
            fee_recipient: pattern.fee_recipient,
            gas_limit: pattern.gas_limit,
            min_value: pattern.min_value,
            reset_relays: pattern.reset_relays,
            created_at: pattern.created_at,
            updated_at: pattern.updated_at,
        }
    }
}

impl From<VouchDefaultRelay> for RelayConfig {
    fn from(relay: VouchDefaultRelay) -> Self {
        Self {
            public_key: relay.public_key,
            fee_recipient: relay.fee_recipient,
            gas_limit: relay.gas_limit,
            min_value: relay.min_value,
        }
    }
}

impl From<VouchProposerRelay> for ProposerRelayConfig {
    fn from(relay: VouchProposerRelay) -> Self {
        Self {
            public_key: relay.public_key,
            fee_recipient: relay.fee_recipient,
            gas_limit: relay.gas_limit,
            min_value: relay.min_value,
            disabled: relay.disabled,
        }
    }
}

impl From<VouchProposerPatternRelay> for RelayConfig {
    fn from(relay: VouchProposerPatternRelay) -> Self {
        Self {
            public_key: relay.public_key,
            fee_recipient: relay.fee_recipient,
            gas_limit: relay.gas_limit,
            min_value: relay.min_value,
        }
    }
}
