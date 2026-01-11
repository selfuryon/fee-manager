-- seed_testdata.sql
-- Generates test data for fee-manager database
-- Usage: psql -d feemanager -f scripts/seed_testdata.sql

-- Enable pgcrypto for gen_random_bytes()
CREATE EXTENSION IF NOT EXISTS pgcrypto;

BEGIN;

-- ============================================================================
-- Helper function to generate random BLS public key (48 bytes = 96 hex chars)
-- ============================================================================
CREATE OR REPLACE FUNCTION generate_bls_pubkey() RETURNS TEXT AS $$
BEGIN
    RETURN '0x' || encode(gen_random_bytes(48), 'hex');
END;
$$ LANGUAGE plpgsql;

-- Helper function to generate random ETH address (20 bytes = 40 hex chars)
CREATE OR REPLACE FUNCTION generate_eth_address() RETURNS TEXT AS $$
BEGIN
    RETURN '0x' || encode(gen_random_bytes(20), 'hex');
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- Mainnet Relays
-- ============================================================================
CREATE TEMP TABLE mainnet_relays (url TEXT, public_key TEXT);
INSERT INTO mainnet_relays VALUES
    ('https://bloxroute.max-profit.blxrbdn.com', '0x8b5d2e73e2a3a55c6c87b8b6eb92e0149a125c852751db1422fa951e42a09b82c142c3ea98d0d9930b056a3bc9896b8f'),
    ('https://bloxroute.regulated.blxrbdn.com', '0xb0b07cd0abef743db4260b0ed50619cf6ad4d82064cb4fbec9d3ec530f7c5e6793d9f286c4e082c0244ffb9f2658fe88'),
    ('https://mainnet-relay.securerpc.com', '0x98650451ba02064f7b000f5768cf0cf4d4e492317d82871bdc87ef841a0743f69f0f1eea11168503240ac35d101c9135'),
    ('https://relay.ultrasound.money', '0xa1559ace749633b997cb3fdacffb890aeebdb0f5a3b6aaa7eeeaf1a38af0a8fe88b9e4b1f61f236d2e64d95733327a62'),
    ('https://aestus.live', '0xa15b52576bcbf1072f4a011c0f99f9fb6c66f3e1ff321f11f461d15e31b1cb359caa092c71bbded0bae5b5ea401aab7e'),
    ('https://agnostic-relay.net', '0xa7ab7a996c8584251c8f925da3170bdfd6ebc75d50f5ddc4050a6fdc77f2a3b5fce2cc750d0865e05d7228af97d69561'),
    ('https://boost-relay.flashbots.net', '0xac6e77dfe25ecd6110b8e780608cce0dab71fdd5ebea22a16c0205200f2f8e2e3ad3b71d3499c54ad14d6c21b41a37ae'),
    ('https://relay.edennetwork.io', '0xb3ee7afcf27f1f1259ac1787876318c6584ee353097a50ed84f51a1f21a323b3736f271a895c7ce918c038e4265918be'),
    ('https://global.titanrelay.xyz', '0x8c4ed5e24fe5c6ae21018437bde147693f68cda427cd1122cf20819c30eda7ed74f72dece09bb313f2a1855595ab677d');

-- ============================================================================
-- Hoodi (Testnet) Relays
-- ============================================================================
CREATE TEMP TABLE hoodi_relays (url TEXT, public_key TEXT);
INSERT INTO hoodi_relays VALUES
    ('https://hoodi.titanrelay.xyz', '0xaa58208899c6105603b74396734a6263cc7d947f444f396a90f7b7d3e65d102aec7e5e5291b27e08d02c50a050825c2f'),
    ('https://hoodi.aestus.live', '0x98f0ef62f00780cf8eb06701a7d22725b9437d4768bb19b363e882ae87129945ec206ec2dc16933f31d983f8225772b6'),
    ('https://boost-relay-hoodi.flashbots.net', '0xafa4c6985aa049fb79dd37010438cfebeb0f2bd42b115b89dd678dab0670c1de38da0c4e9138c9290a398ecd9a0b3110'),
    ('https://bloxroute.hoodi.blxrbdn.com', '0x821f2a65afb70e7f2e820a925a9b4c80a159620582c1766b1b09729fec178b11ea22abb3a51f07b288be815a1a2ff516'),
    ('https://hoodi-builder-proxy-alpha.interstate.so', '0x9110847c15a7f5c80a9fdd5db989a614cc01104e53bd8c252b6f46a4842c7fdef6b9593336035b5094878deff386804c'),
    ('https://hoodi-relay.ethgas.com', '0xb20c3fe59db9c3655088839ef3d972878d182eb745afd8abb1dd2abf6c14f93cd5934ed4446a5fe1ba039e2bc0cf1011'),
    ('https://relay-hoodi.ultrasound.money', '0xb1559beef7b5ba3127485bbbb090362d9f497ba64e177ee2c8e7db74746306efad687f2cf8574e38d70067d40ef136dc');

-- ============================================================================
-- 5 Default Configs
-- ============================================================================
INSERT INTO vouch_default_configs (name, fee_recipient, gas_limit, min_value, active) VALUES
    ('mainnet-default', '0x388c818ca8b9251b393131c08a736a67ccb19297', '30000000', '10000000000000000', true),
    ('mainnet-mev-boost', '0x5e8422345238f34275888049021821e8e08caa1f', '36000000', '50000000000000000', true),
    ('hoodi-testnet', '0x1234567890abcdef1234567890abcdef12345678', '30000000', NULL, true),
    ('mainnet-conservative', '0xdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef', '25000000', '100000000000000000', true),
    ('mainnet-archive', '0xabcdefabcdefabcdefabcdefabcdefabcdefabcd', '30000000', '1000000000000000', false);

-- Relays for mainnet-default (all mainnet relays)
INSERT INTO vouch_default_relays (config_name, url, public_key)
SELECT 'mainnet-default', url, public_key FROM mainnet_relays;

-- Relays for mainnet-mev-boost (only flashbots, ultrasound, bloxroute)
INSERT INTO vouch_default_relays (config_name, url, public_key)
SELECT 'mainnet-mev-boost', url, public_key FROM mainnet_relays
WHERE url LIKE '%flashbots%' OR url LIKE '%ultrasound%' OR url LIKE '%bloxroute%';

-- Relays for hoodi-testnet (all hoodi relays)
INSERT INTO vouch_default_relays (config_name, url, public_key)
SELECT 'hoodi-testnet', url, public_key FROM hoodi_relays;

-- Relays for mainnet-conservative (only flashbots and regulated bloxroute)
INSERT INTO vouch_default_relays (config_name, url, public_key)
SELECT 'mainnet-conservative', url, public_key FROM mainnet_relays
WHERE url LIKE '%flashbots%' OR url LIKE '%regulated%';

-- Relays for mainnet-archive (flashbots, titan, eden)
INSERT INTO vouch_default_relays (config_name, url, public_key)
SELECT 'mainnet-archive', url, public_key FROM mainnet_relays
WHERE url LIKE '%flashbots%' OR url LIKE '%titan%' OR url LIKE '%eden%';

-- ============================================================================
-- 5 Proposer Patterns
-- ============================================================================
INSERT INTO vouch_proposer_patterns (name, pattern, tags, fee_recipient, gas_limit, min_value, reset_relays) VALUES
    ('lido-validators', '^0x8[0-9a-f]{94}$', ARRAY['lido', 'liquid-staking'], '0x388c818ca8b9251b393131c08a736a67ccb19297', '30000000', '20000000000000000', false),
    ('rocketpool-nodes', '^0x9[0-9a-f]{94}$', ARRAY['rocketpool', 'decentralized'], '0x5e8422345238f34275888049021821e8e08caa1f', '32000000', '15000000000000000', false),
    ('coinbase-cloud', '^0xa[0-9a-f]{94}$', ARRAY['coinbase', 'institutional'], '0xc01ba5ec10daddee550000000000000000000000', '28000000', '50000000000000000', true),
    ('solo-stakers', '^0xb[0-9a-f]{94}$', ARRAY['solo', 'home-staker'], NULL, '30000000', '5000000000000000', false),
    ('testnet-validators', '^0x[0-9a-f]{96}$', ARRAY['testnet', 'hoodi'], '0x1234567890abcdef1234567890abcdef12345678', '30000000', NULL, true);

-- Relays for lido-validators (top MEV relays)
INSERT INTO vouch_proposer_pattern_relays (pattern_name, url, public_key)
SELECT 'lido-validators', url, public_key FROM mainnet_relays
WHERE url LIKE '%flashbots%' OR url LIKE '%ultrasound%' OR url LIKE '%bloxroute.max%';

-- Relays for rocketpool-nodes (decentralized relays)
INSERT INTO vouch_proposer_pattern_relays (pattern_name, url, public_key)
SELECT 'rocketpool-nodes', url, public_key FROM mainnet_relays
WHERE url LIKE '%ultrasound%' OR url LIKE '%aestus%' OR url LIKE '%agnostic%';

-- Relays for coinbase-cloud (regulated only)
INSERT INTO vouch_proposer_pattern_relays (pattern_name, url, public_key)
SELECT 'coinbase-cloud', url, public_key FROM mainnet_relays
WHERE url LIKE '%regulated%' OR url LIKE '%flashbots%';

-- Relays for solo-stakers (all available)
INSERT INTO vouch_proposer_pattern_relays (pattern_name, url, public_key)
SELECT 'solo-stakers', url, public_key FROM mainnet_relays;

-- Relays for testnet-validators (hoodi relays)
INSERT INTO vouch_proposer_pattern_relays (pattern_name, url, public_key)
SELECT 'testnet-validators', url, public_key FROM hoodi_relays;

-- ============================================================================
-- 1000 Validators (Proposers)
-- ============================================================================

-- Create temp table with pre-generated validator keys
CREATE TEMP TABLE temp_validators AS
SELECT
    generate_bls_pubkey() as public_key,
    i as idx
FROM generate_series(1, 1000) as i;

-- Insert validators with varied configurations
INSERT INTO vouch_proposers (public_key, fee_recipient, gas_limit, min_value, reset_relays)
SELECT
    v.public_key,
    -- 70% have custom fee_recipient, 30% NULL (use default)
    CASE WHEN v.idx % 10 < 7 THEN generate_eth_address() ELSE NULL END,
    -- Gas limit variations
    CASE
        WHEN v.idx % 5 = 0 THEN '25000000'
        WHEN v.idx % 5 = 1 THEN '30000000'
        WHEN v.idx % 5 = 2 THEN '32000000'
        WHEN v.idx % 5 = 3 THEN '35000000'
        ELSE NULL -- use default
    END,
    -- Min value variations
    CASE
        WHEN v.idx % 4 = 0 THEN '10000000000000000'   -- 0.01 ETH
        WHEN v.idx % 4 = 1 THEN '50000000000000000'   -- 0.05 ETH
        WHEN v.idx % 4 = 2 THEN '100000000000000000'  -- 0.1 ETH
        ELSE NULL -- use default
    END,
    -- 20% have reset_relays = true
    v.idx % 5 = 0
FROM temp_validators v;

-- Add relays for ALL validators (2-5 relays each based on idx pattern)
INSERT INTO vouch_proposer_relays (proposer_public_key, url, public_key, fee_recipient, disabled)
SELECT
    v.public_key,
    r.url,
    r.public_key,
    -- 30% have custom fee_recipient per relay
    CASE WHEN v.idx % 10 < 3 THEN generate_eth_address() ELSE NULL END,
    -- 5% of relays are disabled
    v.idx % 20 = 0
FROM temp_validators v
CROSS JOIN mainnet_relays r
WHERE
    -- Each validator gets 2-5 relays based on idx % 10 pattern
    CASE v.idx % 10
        -- Pattern 0: flashbots + ultrasound + titan (3 relays)
        WHEN 0 THEN r.url LIKE '%flashbots%' OR r.url LIKE '%ultrasound%' OR r.url LIKE '%titan%'
        -- Pattern 1: flashbots + bloxroute (4 relays)
        WHEN 1 THEN r.url LIKE '%flashbots%' OR r.url LIKE '%bloxroute%'
        -- Pattern 2: ultrasound + aestus + agnostic (3 relays)
        WHEN 2 THEN r.url LIKE '%ultrasound%' OR r.url LIKE '%aestus%' OR r.url LIKE '%agnostic%'
        -- Pattern 3: flashbots + eden + securerpc (3 relays)
        WHEN 3 THEN r.url LIKE '%flashbots%' OR r.url LIKE '%eden%' OR r.url LIKE '%securerpc%'
        -- Pattern 4: all bloxroute + titan (3 relays)
        WHEN 4 THEN r.url LIKE '%bloxroute%' OR r.url LIKE '%titan%'
        -- Pattern 5: flashbots + ultrasound + bloxroute.max (3 relays)
        WHEN 5 THEN r.url LIKE '%flashbots%' OR r.url LIKE '%ultrasound%' OR r.url LIKE '%bloxroute.max%'
        -- Pattern 6: aestus + agnostic + eden + titan (4 relays)
        WHEN 6 THEN r.url LIKE '%aestus%' OR r.url LIKE '%agnostic%' OR r.url LIKE '%eden%' OR r.url LIKE '%titan%'
        -- Pattern 7: flashbots + regulated + securerpc (3 relays)
        WHEN 7 THEN r.url LIKE '%flashbots%' OR r.url LIKE '%regulated%' OR r.url LIKE '%securerpc%'
        -- Pattern 8: ultrasound + titan + eden (3 relays)
        WHEN 8 THEN r.url LIKE '%ultrasound%' OR r.url LIKE '%titan%' OR r.url LIKE '%eden%'
        -- Pattern 9: flashbots + ultrasound + aestus + agnostic (4 relays)
        WHEN 9 THEN r.url LIKE '%flashbots%' OR r.url LIKE '%ultrasound%' OR r.url LIKE '%aestus%' OR r.url LIKE '%agnostic%'
    END;

-- ============================================================================
-- Commit-Boost Mux Configs (bonus)
-- ============================================================================
INSERT INTO commit_boost_mux_configs (name) VALUES
    ('lido-set-1'),
    ('rocketpool-set-1'),
    ('mixed-validators');

-- Add some validators to mux configs
INSERT INTO commit_boost_mux_keys (mux_name, public_key)
SELECT 'lido-set-1', public_key FROM temp_validators WHERE idx <= 100;

INSERT INTO commit_boost_mux_keys (mux_name, public_key)
SELECT 'rocketpool-set-1', public_key FROM temp_validators WHERE idx > 100 AND idx <= 200;

INSERT INTO commit_boost_mux_keys (mux_name, public_key)
SELECT 'mixed-validators', public_key FROM temp_validators WHERE idx % 10 = 0;

-- ============================================================================
-- Cleanup helper functions
-- ============================================================================
DROP FUNCTION IF EXISTS generate_bls_pubkey();
DROP FUNCTION IF EXISTS generate_eth_address();

COMMIT;

-- ============================================================================
-- Summary
-- ============================================================================
\echo ''
\echo '=== Seed Data Summary ==='
SELECT 'Default Configs' as entity, COUNT(*) as count FROM vouch_default_configs
UNION ALL
SELECT 'Default Config Relays', COUNT(*) FROM vouch_default_relays
UNION ALL
SELECT 'Proposer Patterns', COUNT(*) FROM vouch_proposer_patterns
UNION ALL
SELECT 'Pattern Relays', COUNT(*) FROM vouch_proposer_pattern_relays
UNION ALL
SELECT 'Proposers (Validators)', COUNT(*) FROM vouch_proposers
UNION ALL
SELECT 'Proposer Relays', COUNT(*) FROM vouch_proposer_relays
UNION ALL
SELECT 'Mux Configs', COUNT(*) FROM commit_boost_mux_configs
UNION ALL
SELECT 'Mux Keys', COUNT(*) FROM commit_boost_mux_keys;
