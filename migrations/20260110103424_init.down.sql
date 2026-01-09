-- Migration Down

-- Drop triggers
DROP TRIGGER IF EXISTS commit_boost_mux_configs_updated_at ON commit_boost_mux_configs;
DROP TRIGGER IF EXISTS vouch_proposer_patterns_updated_at ON vouch_proposer_patterns;
DROP TRIGGER IF EXISTS vouch_proposers_updated_at ON vouch_proposers;
DROP TRIGGER IF EXISTS vouch_default_configs_updated_at ON vouch_default_configs;

-- Drop function
DROP FUNCTION IF EXISTS update_updated_at_column();

-- Drop indexes
DROP INDEX IF EXISTS idx_commit_boost_mux_keys_mux_name;
DROP INDEX IF EXISTS idx_vouch_proposer_patterns_pattern;
DROP INDEX IF EXISTS idx_vouch_proposers_fee_recipient;
DROP INDEX IF EXISTS idx_vouch_default_configs_active;
DROP INDEX IF EXISTS idx_vouch_proposer_patterns_tags;
DROP INDEX IF EXISTS idx_vouch_proposer_pattern_relays_pattern_name;
DROP INDEX IF EXISTS idx_vouch_proposer_relays_proposer_key;
DROP INDEX IF EXISTS idx_vouch_default_relays_config_name;

-- Drop Commit-Boost tables
DROP TABLE IF EXISTS commit_boost_mux_keys;
DROP TABLE IF EXISTS commit_boost_mux_configs;

-- Drop Vouch tables (in reverse order due to foreign keys)
DROP TABLE IF EXISTS vouch_proposer_pattern_relays;
DROP TABLE IF EXISTS vouch_proposer_patterns;
DROP TABLE IF EXISTS vouch_proposer_relays;
DROP TABLE IF EXISTS vouch_proposers;
DROP TABLE IF EXISTS vouch_default_relays;
DROP TABLE IF EXISTS vouch_default_configs;
