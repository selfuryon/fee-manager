// tests/common/mod.rs - Test utilities and helpers
#![allow(dead_code)]

use fee_manager::{config, create_router, run_migrations, AppState};
use reqwest::Client;
use sqlx::PgPool;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::sync::OnceLock;

static TEST_APP: OnceLock<TestApp> = OnceLock::new();
static TEST_COUNTER: AtomicU32 = AtomicU32::new(0);

pub struct TestApp {
    pub address: String,
    pub pool: PgPool,
}

impl TestApp {
    /// Get or create a shared test app instance
    pub async fn get() -> &'static TestApp {
        // Check if already initialized
        if let Some(app) = TEST_APP.get() {
            return app;
        }

        // Load config to get database URL
        let config = config::load_config().expect("Failed to load test config");
        let db_url = config.database.database_url();

        // Create a new runtime for the server in a separate thread
        let (tx, rx) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let address = Self::spawn_inner().await;
                // Create a simple client for health check in this runtime
                let temp_client = Client::new();
                // Wait for server to be ready
                for _ in 0..50 {
                    if temp_client.get(&format!("{}/health", address)).send().await.is_ok() {
                        break;
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                }
                tx.send(address).unwrap();
                // Keep the runtime alive forever
                std::future::pending::<()>().await;
            });
        });

        let address = rx.recv().expect("Failed to receive address");

        // Create pool in the TEST's runtime (current runtime), not server's runtime
        let pool = PgPool::connect(&db_url)
            .await
            .expect("Failed to connect to database for tests");

        TEST_APP.get_or_init(|| TestApp { address, pool })
    }

    /// Create a new HTTP client for this test
    pub fn client(&self) -> Client {
        Client::new()
    }

    /// Generate a unique test ID for this test run
    pub fn unique_id() -> String {
        let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        format!("{:08x}", id)
    }

    async fn spawn_inner() -> String {
        // Load config from environment
        let config = config::load_config().expect("Failed to load test config");

        // Connect to database (this pool is for the SERVER, not for tests)
        let pool = PgPool::connect(&config.database.database_url())
            .await
            .expect("Failed to connect to database");

        // Run migrations
        run_migrations(&pool)
            .await
            .expect("Failed to run migrations");

        // Create app state
        let state = Arc::new(AppState {
            pool,
            config,
        });

        // Create router
        let app = create_router(state);

        // Bind to random port
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("Failed to bind");
        let addr = listener.local_addr().unwrap();

        let address = format!("http://{}", addr);

        // Spawn server in background (within the dedicated runtime)
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        address
    }

    /// Clean up test data (anything with "test_" prefix or "0xdead" prefix for keys)
    pub async fn cleanup_test_data(&self) {
        // Clean in order due to foreign key constraints
        sqlx::query("DELETE FROM vouch_proposer_relays WHERE proposer_public_key LIKE '0xdead%'")
            .execute(&self.pool)
            .await
            .ok();

        sqlx::query("DELETE FROM vouch_proposers WHERE public_key LIKE '0xdead%'")
            .execute(&self.pool)
            .await
            .ok();

        sqlx::query("DELETE FROM vouch_proposer_pattern_relays WHERE pattern_name LIKE 'test_%'")
            .execute(&self.pool)
            .await
            .ok();

        sqlx::query("DELETE FROM vouch_proposer_patterns WHERE name LIKE 'test_%'")
            .execute(&self.pool)
            .await
            .ok();

        sqlx::query("DELETE FROM vouch_default_relays WHERE config_name LIKE 'test_%'")
            .execute(&self.pool)
            .await
            .ok();

        sqlx::query("DELETE FROM vouch_default_configs WHERE name LIKE 'test_%'")
            .execute(&self.pool)
            .await
            .ok();

        sqlx::query("DELETE FROM commit_boost_mux_keys WHERE mux_name LIKE 'test_%'")
            .execute(&self.pool)
            .await
            .ok();

        sqlx::query("DELETE FROM commit_boost_mux_configs WHERE name LIKE 'test_%'")
            .execute(&self.pool)
            .await
            .ok();
    }

    /// Generate a test BLS public key (48 bytes = 96 hex chars after 0x)
    /// Uses "dead" prefix as marker for test keys, followed by suffix padded with zeros on the right
    pub fn test_bls_pubkey(suffix: &str) -> String {
        // Convert suffix to hex if it's not already, then pad to 92 chars (96 - 4 for "dead" prefix)
        // Use left-align (:0<92) so suffix comes first, enabling prefix filtering
        let hex_suffix = suffix.chars()
            .filter(|c| c.is_ascii_hexdigit())
            .collect::<String>();
        format!("0xdead{:0<92}", hex_suffix)
    }

    /// Generate a test ETH address (20 bytes = 40 hex chars after 0x)
    /// Uses "dead" prefix as marker for test addresses
    pub fn test_eth_address(suffix: &str) -> String {
        let hex_suffix = suffix.chars()
            .filter(|c| c.is_ascii_hexdigit())
            .collect::<String>();
        format!("0xdead{:0>36}", hex_suffix)
    }
}
