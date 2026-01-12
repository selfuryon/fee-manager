//! Audit trail logging for admin operations

mod context;

pub use context::RequestContext;

use chrono::{DateTime, Utc};
use serde::Serialize;
use std::fs::OpenOptions;
use std::io::{stderr, stdout, Write};
use std::sync::{Mutex, OnceLock};
use uuid::Uuid;

/// Global audit writer
static AUDIT_WRITER: OnceLock<AuditWriter> = OnceLock::new();

/// Writer for audit events
struct AuditWriter {
    writer: Mutex<Box<dyn Write + Send>>,
}

impl std::fmt::Debug for AuditWriter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuditWriter").finish_non_exhaustive()
    }
}

/// Initialize the audit writer with the specified output destination.
/// Must be called once at startup before any audit logging.
///
/// # Arguments
/// * `output` - "stdout", "stderr", or a file path
pub fn init_audit_writer(output: &str) {
    let writer: Box<dyn Write + Send> = match output {
        "stdout" => Box::new(stdout()),
        "stderr" => Box::new(stderr()),
        path => Box::new(
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .expect("Failed to open audit log file"),
        ),
    };

    AUDIT_WRITER
        .set(AuditWriter {
            writer: Mutex::new(writer),
        })
        .expect("Audit writer already initialized");
}

/// Information about the actor performing an action
#[derive(Debug, Clone, Serialize, Default)]
pub struct ActorInfo {
    pub token_id: Uuid,
    pub token_name: String,
}

/// Types of audit actions
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    Create,
    Update,
    Delete,
    AddKeys,
    RemoveKeys,
}

/// Resource types that can be audited
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceType {
    VouchDefaultConfig,
    VouchProposer,
    VouchProposerPattern,
    CommitBoostMux,
    AuthToken,
}

/// Key field changes to track
#[derive(Debug, Clone, Serialize, Default)]
pub struct AuditChanges {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee_recipient: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_limit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reset_relays: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relays_count: Option<usize>,
}

/// Complete audit event
#[derive(Debug, Clone, Serialize)]
pub struct AuditEvent {
    #[serde(rename = "type")]
    pub event_type: &'static str,
    pub timestamp: DateTime<Utc>,
    pub request_id: Uuid,
    pub actor: ActorInfo,
    pub action: AuditAction,
    pub resource_type: ResourceType,
    pub resource_id: String,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub changes: Option<AuditChanges>,
}

impl AuditEvent {
    /// Create a new successful audit event
    pub fn success(
        request_id: Uuid,
        actor: ActorInfo,
        action: AuditAction,
        resource_type: ResourceType,
        resource_id: impl Into<String>,
    ) -> Self {
        Self {
            event_type: "audit",
            timestamp: Utc::now(),
            request_id,
            actor,
            action,
            resource_type,
            resource_id: resource_id.into(),
            success: true,
            error: None,
            changes: None,
        }
    }

    /// Add changes to the event
    pub fn with_changes(mut self, changes: AuditChanges) -> Self {
        self.changes = Some(changes);
        self
    }

    /// Log this audit event to the configured output
    pub fn log(self) {
        if let Some(writer) = AUDIT_WRITER.get() {
            let json = serde_json::to_string(&self).unwrap_or_default();
            if let Ok(mut w) = writer.writer.lock() {
                let _ = writeln!(w, "{}", json);
            }
        }
    }
}

/// Convenience macro for audit logging
#[macro_export]
macro_rules! audit_log {
    ($ctx:expr, $action:expr, $resource_type:expr, $resource_id:expr) => {
        $crate::audit::AuditEvent::success(
            $ctx.request_id,
            $ctx.actor.clone(),
            $action,
            $resource_type,
            $resource_id,
        )
        .log()
    };
    ($ctx:expr, $action:expr, $resource_type:expr, $resource_id:expr, $changes:expr) => {
        $crate::audit::AuditEvent::success(
            $ctx.request_id,
            $ctx.actor.clone(),
            $action,
            $resource_type,
            $resource_id,
        )
        .with_changes($changes)
        .log()
    };
}
