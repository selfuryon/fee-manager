use config::{Config, Environment, File};
use serde::Deserialize;

#[derive(Clone, Deserialize, Debug)]
pub struct AppConfig {
    pub database: DatabaseConfig,
    pub auth: AuthConfig,
    pub log_level: String,
    /// Log format: "text" (default) or "json"
    #[serde(default = "default_log_format")]
    pub log_format: String,
    /// Enable audit trail logging (default: true)
    #[serde(default = "default_audit_enabled")]
    pub audit_enabled: bool,
    /// Audit output destination: "stdout", "stderr", or file path (default: "stderr")
    #[serde(default = "default_audit_output")]
    pub audit_output: String,
    pub host: String,
    pub port: u16,
}

fn default_log_format() -> String {
    "text".to_string()
}

fn default_audit_enabled() -> bool {
    true
}

fn default_audit_output() -> String {
    "stderr".to_string()
}

#[derive(Clone, Deserialize, Debug, Default)]
pub struct AuthConfig {
    /// Enable authentication for admin routes (default: true)
    #[serde(default = "default_auth_enabled")]
    pub enabled: bool,
}

fn default_auth_enabled() -> bool {
    true
}

impl AppConfig {
    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[derive(Clone, Deserialize, Debug)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub dbname: String,
}

impl DatabaseConfig {
    pub fn database_url(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username, self.password, self.host, self.port, self.dbname
        )
    }
}

pub fn load_config() -> Result<AppConfig, config::ConfigError> {
    let config = Config::builder()
        .add_source(File::with_name("config.yaml"))
        .add_source(Environment::with_prefix("FEE_MANAGER"))
        .build()?;

    config.try_deserialize()
}
