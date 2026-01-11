# Fee Manager - Validator Configuration Service

## Project Overview

REST API microservice in Rust for managing validator configurations for Ethereum staking:

1. **Vouch** - Execution config v2 with default configs, validator-specific overrides, and pattern-based proposer configs with tags
2. **Commit-Boost** - Multiplexer (mux) key sets for simple validator key management

The service allows centralized configuration management.

## Technical Stack

- **Language**: Rust
- **Web Framework**: Axum
- **Database**: PostgreSQL with SQLx
- **Keep it simple**: small microservice without over-engineering

## Core Concepts

### Vouch Integration

**Solution**: Tag and pattern system for execution configs
- Pattern-based proposer configs have tags (e.g., `pool-1`, `high-value`, `relay-A`)
- Request specifies default config name and optional tags via query params
- Response includes: default config + validator-specific configs + pattern-based configs (matched by tags)
- Precedence: validator-specific > default

### Commit-Boost Integration

**Solution**: Named mux configurations
- Simple key sets identified by name
- GET request returns array of validator public keys
- Used for Commit-Boost multiplexer configuration

## Execution Config v2 Structure

Reference: https://github.com/attestantio/vouch/blob/master/docs/executionconfig.md

```json
{
  "version": 2,
  "fee_recipient": "0x...",           // Default fee recipient (optional)
  "gas_limit": "30000000",            // Gas limit (optional, usually omitted)
  "min_value": "0.1",                 // Minimum bid value in ETH (optional)
  "relays": {                         // MEV relays configuration
    "https://relay1.com/": {
      "public_key": "0xac6e...",      // Relay public key for verification
      "fee_recipient": "0x...",       // Can override default
      "gas_limit": "30000000",
      "min_value": "0.2"
    }
  },
  "proposers": [                      // Overrides for specific proposers
    {
      "proposer": "0x8021...8bbe",    // Public key or regex pattern
      "fee_recipient": "0x...",
      "min_value": "0.4",
      "relays": { ... },
      "reset_relays": true,           // Replace all relays instead of merge
      "disabled": true                // Can disable specific relay
    }
  ]
}
```

**Configuration Precedence**: Proposer-specific → Default values → Vouch fallback

## API Structure

See [API_SPEC.md](./API_SPEC.md) for complete API specification.

### Public Endpoints (No Auth)

#### Vouch
```
POST /vouch/v2/execution-config/:config?tags=pool-1,high-value
Body: { "keys": ["0x...", "0x..."] }
Response: { version: 2, fee_recipient: "0x...", relays: {...}, proposers: [...] }
```

**Logic:**
1. Load default config by `:config` name
2. Load proposer-specific configs for keys in request
3. Load proposer patterns matching `?tags` (OR logic)
4. Build response with default + proposer configs + proposer patterns

#### Commit-Boost
```
GET /commit-boost/v1/mux/:name
Response: ["0x...", "0x...", "0x..."]
```

**Logic:**
- Simply return array of validator public keys for the named mux config

### Protected Endpoints (Auth Required)

All protected endpoints use `/api/admin/*` prefix:

**Vouch Management:**
- `/api/admin/vouch/proposers` - CRUD for proposer-specific configs (validator public_key + config + relays)
- `/api/admin/vouch/configs/default` - CRUD for named default configs with relays
- `/api/admin/vouch/proposer-patterns` - CRUD for pattern-based proposer configs with tags and relays

**Commit-Boost Management:**
- `/api/admin/commit-boost/mux` - CRUD for mux configs
- `/api/admin/commit-boost/mux/:name/keys` - Add/remove keys from mux

## Data Model Overview

### Database Schema

See [schema.sql](./schema.sql) for complete PostgreSQL schema.

### Vouch Tables

- **vouch_default_configs**: Named default configs (PK: name)
  - Fields: name, fee_recipient, gas_limit, min_value, active, timestamps

- **vouch_default_relays**: Relays for default configs (FK: config_name)
  - Fields: url, public_key, fee_recipient, gas_limit, min_value

- **vouch_proposers**: Proposer-specific configs (PK: public_key)
  - Fields: public_key, fee_recipient, gas_limit, min_value, reset_relays, timestamps

- **vouch_proposer_relays**: Relays for proposers (FK: proposer_public_key)
  - Fields: url, public_key, fee_recipient, gas_limit, min_value, disabled

- **vouch_proposer_patterns**: Pattern configs with tags (PK: name)
  - Fields: name, pattern, tags (TEXT[]), fee_recipient, gas_limit, min_value, reset_relays, timestamps
  - GIN index on tags for fast searches

- **vouch_proposer_pattern_relays**: Relays for patterns (FK: pattern_name)
  - Fields: url, public_key, fee_recipient, gas_limit, min_value

### Commit-Boost Tables

- **commit_boost_mux_configs**: Mux configs (PK: name)
  - Fields: name, timestamps

- **commit_boost_mux_keys**: Keys in mux (FK: mux_name)
  - Fields: mux_name, public_key
  - Unique constraint: (mux_name, public_key)

## Key Design Decisions

1. **Tags only on proposer patterns**: Proposers (specific public keys) don't have tags. Tags are used to group pattern-based proposer configs that can be included via query params.

2. **Proposer patterns have unique names**: Each proposer pattern has a unique name for identification. Tags are not unique - multiple patterns can share tags.
   - Example: `pool1-mainnet` (tags: `pool-1`, `high-value`) and `pool1-backup` (tags: `pool-1`, `backup`) both share the `pool-1` tag
   - Request with `?tags=pool-1` will include both patterns in the response

3. **Tag matching is OR logic**: `?tags=pool-1,high-value` includes patterns with tag `pool-1` OR `high-value`

4. **Separate paths for services**: `/vouch/*` and `/commit-boost/*` for public API, `/api/admin/vouch/*` and `/api/admin/commit-boost/*` for management

5. **Config name in path**: Default config specified as path parameter (e.g., `/vouch/v2/execution-config/main`) instead of query param

6. **Structured request body**: Request body is `{"keys": [...]}` not just raw array

7. **Rich filtering**: All list endpoints support filtering via query parameters for all fields (string prefix/exact match, numeric exact match, boolean true/false, tags array contains)

## Testing with Vouch

```bash
# Vouch checks configuration for a validator
vouch --proposer-config-check 0x8021...8bbe | jq .
```

## Database Notes

- **Schema**: PostgreSQL with SQLx (see schema.sql)
- **Timestamps**: Auto-managed via triggers (created_at, updated_at)
- **Cascading deletes**: Relays automatically deleted when parent config is deleted
- **Unique constraints**: Prevent duplicate relays per config, duplicate keys per mux
- **GIN index**: Fast tag searches on proposer patterns using ANY operator
- **Array type**: tags stored as TEXT[] in proposer patterns

## General Notes

- Version is always `2` (execution config v2)
- Regex patterns in proposer patterns follow Vouch format (e.g., `^Pool1/.*$`)
- `reset_relays: true` means complete replacement of relays instead of merge
- `disabled: true` on relay means it's disabled
- Unknown proposers (validators not in DB) are not included in response - Vouch will apply default config for them
- Proposers are identified by validator public_key and include full configuration inline

## Development Checklist

When adding new routes:

1. **OpenAPI schema** (`src/openapi.rs`):
   - Add handler paths to `paths(...)` section
   - Add request/response schemas to `components(schemas(...))` section
   - Add new tag if needed to `tags(...)` section

2. **Tests** (`tests/`):
   - Add integration tests for new endpoints
   - Use `TestApp::client()` for authenticated requests
   - Use `TestApp::client_unauthenticated()` for public routes or auth failure tests

3. **Run checks**:
   - `cargo test` - all tests must pass
   - `cargo sqlx prepare` - update offline query cache if new SQL queries added
