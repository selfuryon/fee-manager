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

- Rust 1.75+
- PostgreSQL 14+
- Docker/Podman (optional, for database)

### Configuration

Create `config.yaml` in the project root:

```yaml
database:
  host: localhost
  port: 5432
  username: postgres
  password: postgres
  dbname: fee_manager

log_level: info
host: 0.0.0.0
port: 3000
```

Environment variables can override config values with `FEE_MANAGER_` prefix:

```bash
export FEE_MANAGER_DATABASE__HOST=localhost
export FEE_MANAGER_DATABASE__PASSWORD=secret
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

## API Endpoints

### Public Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/vouch/v2/execution-config/{config}` | Get execution config for Vouch |
| GET | `/commit-boost/v1/mux/{name}` | Get validator keys for Commit-Boost |

### Admin Endpoints (Protected)

#### Vouch - Default Configs

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/admin/vouch/configs/default` | List default configs |
| POST | `/api/admin/vouch/configs/default` | Create default config |
| GET | `/api/admin/vouch/configs/default/{name}` | Get default config |
| PUT | `/api/admin/vouch/configs/default/{name}` | Update default config |
| DELETE | `/api/admin/vouch/configs/default/{name}` | Delete default config |

#### Vouch - Proposers

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/admin/vouch/proposers` | List proposers |
| GET | `/api/admin/vouch/proposers/{public_key}` | Get proposer |
| PUT | `/api/admin/vouch/proposers/{public_key}` | Create/update proposer |
| DELETE | `/api/admin/vouch/proposers/{public_key}` | Delete proposer |

#### Vouch - Proposer Patterns

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/admin/vouch/proposer-patterns` | List patterns |
| POST | `/api/admin/vouch/proposer-patterns` | Create pattern |
| GET | `/api/admin/vouch/proposer-patterns/{name}` | Get pattern |
| PUT | `/api/admin/vouch/proposer-patterns/{name}` | Update pattern |
| DELETE | `/api/admin/vouch/proposer-patterns/{name}` | Delete pattern |

#### Commit-Boost - Mux Configs

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/admin/commit-boost/mux` | List mux configs |
| POST | `/api/admin/commit-boost/mux` | Create mux config |
| GET | `/api/admin/commit-boost/mux/{name}` | Get mux config |
| PUT | `/api/admin/commit-boost/mux/{name}` | Update mux config |
| DELETE | `/api/admin/commit-boost/mux/{name}` | Delete mux config |
| POST | `/api/admin/commit-boost/mux/{name}/keys` | Add keys to mux |
| DELETE | `/api/admin/commit-boost/mux/{name}/keys` | Remove keys from mux |

### Health Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/ready` | Readiness probe |
| GET | `/health` | Health check |

## API Documentation

Swagger UI is available at `/swagger-ui` when the service is running.

## Usage Examples

### Get Execution Config (Vouch)

```bash
curl -X POST "http://localhost:3000/vouch/v2/execution-config/main?tags=pool-1,high-value" \
  -H "Content-Type: application/json" \
  -d '{"keys": ["0x8021...8bbe", "0xa123...def4"]}'
```

### Get Mux Keys (Commit-Boost)

```bash
curl "http://localhost:3000/commit-boost/v1/mux/pool-1"
```

### Create Default Config

```bash
curl -X POST "http://localhost:3000/api/admin/vouch/configs/default" \
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
