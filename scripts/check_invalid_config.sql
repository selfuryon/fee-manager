-- check_invalid_config.sql
-- Lists stored values that the admin API's write validation would now reject.
-- Read-only. Run before deploying the validation change; an empty result means
-- nothing stored would have been refused.
-- Usage: psql -d feemanager -f scripts/check_invalid_config.sql
--
-- The rules mirror src/validation.rs:
--   gas_limit  digits only, > 0, fits in u64
--   min_value  digits with at most one decimal point, no sign or exponent
--   relay URL  http(s) scheme followed by a host
--   pattern    only checked for being non-empty: whether it compiles can only
--              be checked by the service itself
--   proposer key  lower-case 0x + 96 hex digits; any other form is never matched
--              by the keys Vouch sends (re-create it lower-case, then DELETE the old row)

WITH fields (source, owner, field, value) AS (
    SELECT 'vouch_default_configs', name, 'gas_limit', gas_limit FROM vouch_default_configs
    UNION ALL SELECT 'vouch_default_configs', name, 'min_value', min_value FROM vouch_default_configs
    UNION ALL SELECT 'vouch_proposers', public_key, 'gas_limit', gas_limit FROM vouch_proposers
    UNION ALL SELECT 'vouch_proposers', public_key, 'min_value', min_value FROM vouch_proposers
    UNION ALL SELECT 'vouch_proposer_patterns', name, 'gas_limit', gas_limit FROM vouch_proposer_patterns
    UNION ALL SELECT 'vouch_proposer_patterns', name, 'min_value', min_value FROM vouch_proposer_patterns
    UNION ALL SELECT 'vouch_proposer_patterns', name, 'pattern', pattern FROM vouch_proposer_patterns
    UNION ALL SELECT 'vouch_default_relays', config_name, 'url', url FROM vouch_default_relays
    UNION ALL SELECT 'vouch_default_relays', config_name || ' ' || url, 'gas_limit', gas_limit FROM vouch_default_relays
    UNION ALL SELECT 'vouch_default_relays', config_name || ' ' || url, 'min_value', min_value FROM vouch_default_relays
    UNION ALL SELECT 'vouch_proposer_relays', proposer_public_key, 'url', url FROM vouch_proposer_relays
    UNION ALL SELECT 'vouch_proposer_relays', proposer_public_key || ' ' || url, 'gas_limit', gas_limit FROM vouch_proposer_relays
    UNION ALL SELECT 'vouch_proposer_relays', proposer_public_key || ' ' || url, 'min_value', min_value FROM vouch_proposer_relays
    UNION ALL SELECT 'vouch_proposer_pattern_relays', pattern_name, 'url', url FROM vouch_proposer_pattern_relays
    UNION ALL SELECT 'vouch_proposer_pattern_relays', pattern_name || ' ' || url, 'gas_limit', gas_limit FROM vouch_proposer_pattern_relays
    UNION ALL SELECT 'vouch_proposer_pattern_relays', pattern_name || ' ' || url, 'min_value', min_value FROM vouch_proposer_pattern_relays
    UNION ALL SELECT 'vouch_proposers', public_key, 'public_key', public_key FROM vouch_proposers
    UNION ALL SELECT 'vouch_proposer_relays', proposer_public_key || ' ' || url, 'proposer_public_key', proposer_public_key FROM vouch_proposer_relays
)
SELECT source, owner, field, value
FROM fields
WHERE value IS NOT NULL
  AND CASE field
        -- Nested CASE: OR does not short-circuit, and the cast fails on non-digits
        WHEN 'gas_limit' THEN CASE WHEN value !~ '^[0-9]+$' THEN true
                                   ELSE value::numeric = 0 OR value::numeric > 18446744073709551615
                              END
        WHEN 'min_value' THEN value !~ '^[0-9]+(\.[0-9]+)?$'
        WHEN 'url'       THEN value !~* '^https?://[^/?#[:space:]]+'
        WHEN 'pattern'   THEN value = ''
        WHEN 'public_key' THEN value !~ '^0x[0-9a-f]{96}$'
        WHEN 'proposer_public_key' THEN value !~ '^0x[0-9a-f]{96}$'
      END
ORDER BY source, owner, field;
