# Fee Manager - Validator Configuration Service

## Project Overview

REST API microservice in Rust for managing validator configurations for Ethereum staking:

1. **Vouch** - Execution config v2 with default configs, validator-specific overrides, and pattern-based proposer configs with tags
2. **Commit-Boost** - Multiplexer (mux) key sets for simple validator key management

The service allows centralized configuration management.

**Keep it simple**: small microservice without over-engineering.

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

The API contract is [openapi.json](./openapi.json), generated from the utoipa annotations; semantics are in the README.

### Public Endpoints (No Auth)

#### Vouch
```
POST /vouch/v2/execution-config/:config?tags=pool-1,high-value
Body: ["0x...", "0x..."]
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

All management endpoints live under `/api/admin/vouch/*` and `/api/admin/commit-boost/*`.

## Key Design Decisions

1. **Tags only on proposer patterns**: Proposers (specific public keys) don't have tags. Tags are used to group pattern-based proposer configs that can be included via query params.

2. **Proposer patterns have unique names**: Each proposer pattern has a unique name for identification. Tags are not unique - multiple patterns can share tags.
   - Example: `pool1-mainnet` (tags: `pool-1`, `high-value`) and `pool1-backup` (tags: `pool-1`, `backup`) both share the `pool-1` tag
   - Request with `?tags=pool-1` will include both patterns in the response

3. **Tag matching is OR logic**: `?tags=pool-1,high-value` includes patterns with tag `pool-1` OR `high-value`

4. **Separate paths for services**: `/vouch/*` and `/commit-boost/*` for public API, `/api/admin/vouch/*` and `/api/admin/commit-boost/*` for management

5. **Config name in path**: Default config specified as path parameter (e.g., `/vouch/v2/execution-config/main`) instead of query param

6. **Raw array request body**: the execution-config body is a plain JSON array of public keys, not `{"keys": [...]}`

7. **Rich filtering**: All list endpoints support filtering via query parameters for all fields (string prefix/exact match, numeric exact match, boolean true/false, tags array contains)

8. **Keyset pagination**: lists take `limit` + `after` and return `{items, next_cursor}` (`src/pagination.rs`), ordered by primary key — same contract as kleido. No offset, no total.

9. **PUT replaces**: every `PUT` is a full replacement; omitted fields are cleared. Don't reintroduce `COALESCE`-style partial updates.

10. **Extractors**: handlers use `AppJson`/`AppQuery`/`AppPath` (`src/extract.rs`), never Axum's `Json`/`Query`/`Path` for input — the wrappers turn rejections into the JSON error body. `IntoParams` structs need `#[into_params(parameter_in = Query)]`, since utoipa can no longer infer it.

## Testing with Vouch

```bash
# Vouch checks configuration for a validator
vouch --proposer-config-check 0x8021...8bbe | jq .
```

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
   - Regenerate `openapi.json`: `UPDATE_OPENAPI=1 cargo test --test openapi_test` (any API change, not just new routes)

2. **Tests** (`tests/`):
   - Add integration tests for new endpoints
   - Use `TestApp::client()` for authenticated requests
   - Use `TestApp::client_unauthenticated()` for public routes or auth failure tests

3. **Run checks**:
   - `cargo test` - all tests must pass
   - `cargo sqlx prepare -- --all-targets` - update offline query cache if new SQL queries added; CI and the clippy hook compile with `SQLX_OFFLINE=true`, so a missing entry fails there

## Spec-Driven Changes (OpenSpec)

Non-trivial behaviour changes go through OpenSpec: `/opsx:propose` drafts `openspec/changes/<name>/` (proposal, spec delta, design, tasks), `/opsx:apply` implements it, `/opsx:archive` merges the delta into `openspec/specs/`. Project context and per-artifact rules live in `openspec/config.yaml`. The `.claude/commands/opsx/` and `.claude/skills/openspec-*` files are generated — refresh them with `openspec update`, don't edit by hand.
