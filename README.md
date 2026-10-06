# Fee Manager

REST API service for managing validator configurations for Ethereum staking infrastructure.

## Overview

Fee Manager provides centralized configuration management for:

- **Vouch** - Execution configurations with default configs, validator-specific overrides, and pattern-based proposer configs with tag support
- **Commit-Boost** - Validator key sets for multiplexer configuration

## Features

- Normalized PostgreSQL schema with type-safe SQLx queries
- Public endpoints for Vouch and Commit-Boost integration
- Protected admin API for configuration management
- Tag-based configuration grouping with OR logic
- Pattern-based proposer configs using regex matching
- OpenAPI/Swagger documentation
- Structured logging with tracing

## Tech Stack

- **Language**: Rust
- **Web Framework**: Axum 0.8
- **Database**: PostgreSQL 14+ with SQLx
- **Documentation**: utoipa + Swagger UI

## Getting Started

### Prerequisites

- Rust 1.90+
- PostgreSQL 14+
- Docker/Podman (optional)

### Configuration

Create `config.yaml` in the project root:

```yaml
database:
  host: localhost
  port: 5432
  username: postgres
  password: postgres
  dbname: fee_manager

auth:
  enabled: true          # Enable Bearer token auth for admin routes

log_level: info
log_format: text         # "text" or "json"
audit_enabled: true      # Enable audit trail logging
audit_output: stderr     # "stdout", "stderr", or file path

host: 0.0.0.0
port: 3000
```

Environment variables can override config values with `FEE_MANAGER_` prefix:

```bash
export FEE_MANAGER_DATABASE__HOST=localhost
export FEE_MANAGER_DATABASE__PASSWORD=secret
export FEE_MANAGER_AUTH__ENABLED=true
```

### Database Setup

```bash
# Start PostgreSQL (example with Docker)
docker run -d \
  --name fee-manager-db \
  -e POSTGRES_USER=postgres \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=fee_manager \
  -p 5432:5432 \
  postgres:16

# Migrations run automatically on startup
```

### Running

```bash
# Development
cargo run

# Production
cargo build --release
./target/release/fee-manager
```

The service will be available at `http://localhost:3000`.

### Container Build

```bash
# Build image
docker build -f Containerfile -t fee-manager .

# Run with config file
docker run -p 3000:3000 -v ./config.yaml:/app/config.yaml fee-manager
```

## Authentication

Admin endpoints (`/api/admin/*`) require Bearer token authentication:

```
Authorization: Bearer <token>
```

Tokens are stored in the database. On first startup, a default token is auto-generated and printed to the logs. Additional tokens can be managed via the admin API.

## API Endpoints

### Public Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/vouch/v2/execution-config/{config}` | Get execution config for Vouch |
| GET | `/commit-boost/v1/mux/{name}` | Get validator keys for Commit-Boost |

### Admin Endpoints (Protected)

#### Auth Tokens

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/admin/tokens` | List tokens |
| POST | `/api/admin/tokens` | Create token |
| DELETE | `/api/admin/tokens/{id}` | Delete token |

#### Vouch - Default Configs

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/admin/vouch/configs/default` | List default configs |
| POST | `/api/admin/vouch/configs/default` | Create default config |
| GET | `/api/admin/vouch/configs/default/{name}` | Get default config |
| PUT | `/api/admin/vouch/configs/default/{name}` | Replace default config |
| DELETE | `/api/admin/vouch/configs/default/{name}` | Delete default config |

#### Vouch - Proposers

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/admin/vouch/proposers` | List proposers |
| GET | `/api/admin/vouch/proposers/{public_key}` | Get proposer |
| PUT | `/api/admin/vouch/proposers/{public_key}` | Create or replace proposer |
| DELETE | `/api/admin/vouch/proposers/{public_key}` | Delete proposer |

#### Vouch - Proposer Patterns

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/admin/vouch/proposer-patterns` | List patterns |
| POST | `/api/admin/vouch/proposer-patterns` | Create pattern |
| GET | `/api/admin/vouch/proposer-patterns/{name}` | Get pattern |
| PUT | `/api/admin/vouch/proposer-patterns/{name}` | Replace pattern |
| DELETE | `/api/admin/vouch/proposer-patterns/{name}` | Delete pattern |

#### Commit-Boost - Mux Configs

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/admin/commit-boost/mux` | List mux configs |
| POST | `/api/admin/commit-boost/mux` | Create mux config |
| GET | `/api/admin/commit-boost/mux/{name}` | Get mux config |
| PUT | `/api/admin/commit-boost/mux/{name}` | Replace mux config keys |
| DELETE | `/api/admin/commit-boost/mux/{name}` | Delete mux config |
| POST | `/api/admin/commit-boost/mux/{name}/keys` | Add keys to mux |
| DELETE | `/api/admin/commit-boost/mux/{name}/keys` | Remove keys from mux |

### Health Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/ready` | Readiness probe |
| GET | `/health` | Health check |

## API Documentation

The API contract is [`openapi.json`](./openapi.json), generated from the handlers' utoipa annotations. A test fails when it drifts from the code; regenerate it with:

```bash
UPDATE_OPENAPI=1 cargo test --test openapi_test
```

Swagger UI is available at `/swagger-ui` when the service is running.

### Execution config semantics

`POST /vouch/v2/execution-config/{config}` takes a plain JSON array of validator public keys and returns a Vouch [execution config v2](https://github.com/attestantio/vouch/blob/master/docs/executionconfig.md):

1. Top-level `fee_recipient`, `gas_limit`, `min_value` and `relays` come from the default config named by `{config}` (`404` if it does not exist).
2. `proposers` lists the configs of the requested keys that are known to the service. Unknown keys are omitted, so Vouch applies the defaults to them.
3. `proposers` then lists the pattern configs whose tags match `?tags=` (OR logic), ordered by the position of their first matching tag in the request.

Vouch uses the first `proposers` entry that matches, so validator-specific entries take precedence over patterns. Within an entry, an omitted field falls back to the default. Relays are merged with the default relays unless `reset_relays: true`; a proposer relay marked `disabled: true` is passed through and Vouch skips it.

### Accepted values

Admin create and update requests are validated before anything is stored, including the fields of every relay; an invalid value fails the whole request with `400` `INVALID_DATA` and a message naming the field:

| Field | Accepted |
|-------|----------|
| `gas_limit` | positive integer in decimal digits, e.g. `"30000000"` |
| `min_value` | non-negative decimal amount of ETH, e.g. `"0"`, `"0.1"` (no sign, exponent or bare `.5`) |
| relay URL (`relays` key) | absolute `http`/`https` URL with a host; stored exactly as sent, trailing slash included |
| `pattern` | a regular expression that compiles (RE2 syntax, as Vouch uses) |

Values are stored as sent, without normalization.

### List filters

Vouch list endpoints accept a query parameter per field, combined with AND: prefix match for names, public keys and relay URLs; substring match for `pattern`; exact match for everything else; `tag` matches patterns that carry that tag; `relay_*` filters match configs that have at least one such relay.

### Pagination

All admin lists (proposers, default configs, proposer patterns, mux configs) use keyset pagination: `limit` (1–1000, default 100) and an opaque `after` cursor. The response is

```json
{ "items": [ ... ], "next_cursor": "..." }
```

Pass `next_cursor` as `after` to get the next page; it is absent on the last page. Items are ordered by identifier (public key for proposers, name otherwise), so a walk visits every item exactly once even while others are being added. There is no total count.

```bash
curl -H "Authorization: Bearer $TOKEN" "http://localhost:3000/api/admin/vouch/proposers?limit=1000"
curl -H "Authorization: Bearer $TOKEN" "http://localhost:3000/api/admin/vouch/proposers?limit=1000&after=<next_cursor>"
```

### Updates replace

`PUT` replaces the whole resource. Omitted optional fields are cleared, an omitted `relays` removes all relays, an omitted `tags` empties them, and `active` / `reset_relays` fall back to `true` / `false`. To change one field, read the resource, modify it, and `PUT` it back in full.

Proposer keys in the path may be sent in either hex case; they are stored and returned in lower case, the form Vouch sends.

### Errors

Every error response, including unknown routes, has the body `{"error": {"code": "...", "message": "..."}}`:

| Status | `code` | When |
|--------|--------|------|
| 400 | `INVALID_JSON` | body is not valid JSON or not sent as JSON |
| 400 | `INVALID_DATA` | a field is missing, has the wrong type or an invalid value (including keys and addresses), or a path parameter is invalid |
| 400 | `INVALID_QUERY` | a query parameter cannot be parsed or is out of range |
| 401 | `UNAUTHORIZED` | missing or invalid admin token |
| 404 | `NOT_FOUND` | unknown resource or route |
| 405 | `METHOD_NOT_ALLOWED` | method not supported on the route |
| 409 | `CONFLICT` | creating a default config, pattern or mux config whose name exists |
| 413 | `PAYLOAD_TOO_LARGE` | body over the limit: 10 MB for the execution-config endpoint (about 100,000 keys), 2 MB elsewhere |
| 500 | `INTERNAL_ERROR` | unexpected failure |

## Usage Examples

### Get Execution Config (Vouch)

```bash
curl -X POST "http://localhost:3000/vouch/v2/execution-config/main?tags=pool-1,high-value" \
  -H "Content-Type: application/json" \
  -d '["0x8021...8bbe", "0xa123...def4"]'
```

### Get Mux Keys (Commit-Boost)

```bash
curl "http://localhost:3000/commit-boost/v1/mux/pool-1"
```

### Create Default Config

```bash
curl -X POST "http://localhost:3000/api/admin/vouch/configs/default" \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "main",
    "fee_recipient": "0x1234...5678",
    "gas_limit": "30000000",
    "min_value": "0.1",
    "active": true,
    "relays": {
      "https://relay1.example.com/": {
        "public_key": "0xac6e77..."
      }
    }
  }'
```

### Create Proposer Pattern

```bash
curl -X POST "http://localhost:3000/api/admin/vouch/proposer-patterns" \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "pool1-mainnet",
    "pattern": "^Pool1/.*$",
    "tags": ["pool-1", "high-value"],
    "fee_recipient": "0x7777...2222",
    "reset_relays": true
  }'
```

## Database Schema

The service uses a normalized PostgreSQL schema with the following tables:

**Auth:**
- `auth_tokens` - API tokens for admin authentication

**Vouch:**
- `vouch_default_configs` - Named default configurations
- `vouch_default_relays` - Relays for default configs
- `vouch_proposers` - Validator-specific configurations
- `vouch_proposer_relays` - Relays for proposers
- `vouch_proposer_patterns` - Pattern-based configurations with tags
- `vouch_proposer_pattern_relays` - Relays for patterns

**Commit-Boost:**
- `commit_boost_mux_configs` - Named mux configurations
- `commit_boost_mux_keys` - Validator keys in mux configs

## License

MIT
