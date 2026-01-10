-- clean_testdata.sql
-- Removes all test data from fee-manager database
-- Usage: psql -d feemanager -f scripts/clean_testdata.sql

BEGIN;

-- Order matters due to foreign keys (child tables first)
TRUNCATE vouch_proposer_relays CASCADE;
TRUNCATE vouch_proposers CASCADE;
TRUNCATE vouch_proposer_pattern_relays CASCADE;
TRUNCATE vouch_proposer_patterns CASCADE;
TRUNCATE vouch_default_relays CASCADE;
TRUNCATE vouch_default_configs CASCADE;
TRUNCATE commit_boost_mux_keys CASCADE;
TRUNCATE commit_boost_mux_configs CASCADE;

COMMIT;

\echo 'All test data cleaned!'
