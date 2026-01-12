// models.rs - Database models for normalized schema
use crate::addresses::{BlsPubkey, EthAddress};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// ============================================================================
// Vouch - Default Configs
// ============================================================================

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct VouchDefaultConfig {
    pub name: String,
    pub fee_recipient: Option<EthAddress>,
    pub gas_limit: Option<String>,
    pub min_value: Option<String>,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct VouchDefaultRelay {
    pub id: i32,
    pub config_name: String,
    pub url: String,
    pub public_key: BlsPubkey,
    pub fee_recipient: Option<EthAddress>,
    pub gas_limit: Option<String>,
    pub min_value: Option<String>,
}

// ============================================================================
// Vouch - Proposers
// ============================================================================

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct VouchProposer {
    pub public_key: BlsPubkey,
    pub fee_recipient: Option<EthAddress>,
    pub gas_limit: Option<String>,
    pub min_value: Option<String>,
    pub reset_relays: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct VouchProposerRelay {
    pub id: i32,
    pub proposer_public_key: BlsPubkey,
    pub url: String,
    pub public_key: BlsPubkey,
    pub fee_recipient: Option<EthAddress>,
    pub gas_limit: Option<String>,
    pub min_value: Option<String>,
    pub disabled: bool,
}

// ============================================================================
// Vouch - Proposer Patterns
// ============================================================================

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct VouchProposerPattern {
    pub name: String,
    pub pattern: String,
    pub tags: Vec<String>,
    pub fee_recipient: Option<EthAddress>,
    pub gas_limit: Option<String>,
    pub min_value: Option<String>,
    pub reset_relays: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct VouchProposerPatternRelay {
    pub id: i32,
    pub pattern_name: String,
    pub url: String,
    pub public_key: BlsPubkey,
    pub fee_recipient: Option<EthAddress>,
    pub gas_limit: Option<String>,
    pub min_value: Option<String>,
    pub disabled: bool,
}

// ============================================================================
// Commit-Boost - Mux Configs
// ============================================================================

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct CommitBoostMuxConfig {
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct CommitBoostMuxKey {
    pub id: i32,
    pub mux_name: String,
    pub public_key: BlsPubkey,
}
