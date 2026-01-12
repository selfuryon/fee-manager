-- Migration Up
-- Fee Manager Database Schema

-- ============================================================================
-- Vouch Tables
-- ============================================================================

-- Default Configs (named configurations like "main", "testnet")
CREATE TABLE vouch_default_configs (
    name TEXT PRIMARY KEY,
    fee_recipient TEXT,
    gas_limit TEXT,
    min_value TEXT,
    active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Relays for default configs
CREATE TABLE vouch_default_relays (
    id SERIAL PRIMARY KEY,
    config_name TEXT NOT NULL REFERENCES vouch_default_configs(name) ON DELETE CASCADE,
    url TEXT NOT NULL,
    public_key TEXT NOT NULL,
    fee_recipient TEXT,
    gas_limit TEXT,
    min_value TEXT,
    UNIQUE(config_name, url)
);

-- Proposers (validator-specific configurations)
CREATE TABLE vouch_proposers (
    public_key TEXT PRIMARY KEY,
    fee_recipient TEXT,
    gas_limit TEXT,
    min_value TEXT,
    reset_relays BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Relays for proposers
CREATE TABLE vouch_proposer_relays (
    id SERIAL PRIMARY KEY,
    proposer_public_key TEXT NOT NULL REFERENCES vouch_proposers(public_key) ON DELETE CASCADE,
    url TEXT NOT NULL,
    public_key TEXT NOT NULL,
    fee_recipient TEXT,
    gas_limit TEXT,
    min_value TEXT,
    disabled BOOLEAN NOT NULL DEFAULT false,
    UNIQUE(proposer_public_key, url)
);

-- Proposer Patterns (pattern-based configurations with tags)
CREATE TABLE vouch_proposer_patterns (
    name TEXT PRIMARY KEY,
    pattern TEXT NOT NULL,
    tags TEXT[] NOT NULL DEFAULT '{}',
    fee_recipient TEXT,
    gas_limit TEXT,
    min_value TEXT,
    reset_relays BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Relays for proposer patterns
CREATE TABLE vouch_proposer_pattern_relays (
    id SERIAL PRIMARY KEY,
    pattern_name TEXT NOT NULL REFERENCES vouch_proposer_patterns(name) ON DELETE CASCADE,
    url TEXT NOT NULL,
    public_key TEXT NOT NULL,
    fee_recipient TEXT,
    gas_limit TEXT,
    min_value TEXT,
    disabled BOOLEAN NOT NULL DEFAULT false,
    UNIQUE(pattern_name, url)
);

-- ============================================================================
-- Commit-Boost Tables
-- ============================================================================

-- Mux Configs (named key sets for multiplexer)
CREATE TABLE commit_boost_mux_configs (
    name TEXT PRIMARY KEY,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Validator keys in mux configs
CREATE TABLE commit_boost_mux_keys (
    id SERIAL PRIMARY KEY,
    mux_name TEXT NOT NULL REFERENCES commit_boost_mux_configs(name) ON DELETE CASCADE,
    public_key TEXT NOT NULL,
    UNIQUE(mux_name, public_key)
);

-- ============================================================================
-- Indexes
-- ============================================================================

-- Vouch indexes
CREATE INDEX idx_vouch_default_relays_config_name ON vouch_default_relays(config_name);
CREATE INDEX idx_vouch_proposer_relays_proposer_key ON vouch_proposer_relays(proposer_public_key);
CREATE INDEX idx_vouch_proposer_pattern_relays_pattern_name ON vouch_proposer_pattern_relays(pattern_name);

-- GIN index for fast tag searches
CREATE INDEX idx_vouch_proposer_patterns_tags ON vouch_proposer_patterns USING GIN(tags);

-- Filtering indexes
CREATE INDEX idx_vouch_default_configs_active ON vouch_default_configs(active) WHERE active = true;
CREATE INDEX idx_vouch_proposers_fee_recipient ON vouch_proposers(fee_recipient) WHERE fee_recipient IS NOT NULL;
CREATE INDEX idx_vouch_proposer_patterns_pattern ON vouch_proposer_patterns(pattern);

-- Commit-Boost indexes
CREATE INDEX idx_commit_boost_mux_keys_mux_name ON commit_boost_mux_keys(mux_name);

-- ============================================================================
-- Triggers for automatic updated_at
-- ============================================================================

CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER vouch_default_configs_updated_at
    BEFORE UPDATE ON vouch_default_configs
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER vouch_proposers_updated_at
    BEFORE UPDATE ON vouch_proposers
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER vouch_proposer_patterns_updated_at
    BEFORE UPDATE ON vouch_proposer_patterns
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER commit_boost_mux_configs_updated_at
    BEFORE UPDATE ON commit_boost_mux_configs
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
