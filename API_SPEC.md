# Fee Manager API Specification

## Overview

REST API for managing validator configurations for:
1. **Vouch** - Execution configurations with default configs, validator-specific overrides, and pattern-based proposer configs
2. **Commit-Boost** - Simple validator key sets for multiplexer configuration

## Technical Stack

- **Database**: PostgreSQL 14+ (via Docker/Podman)
- **ORM**: SQLx for type-safe SQL queries
- **Schema**: See [schema.sql](./schema.sql) for complete database schema

## Authentication

- **Public endpoints**: No authentication required
- **Protected endpoints** (`/api/admin/*`): Authentication required (implementation TBD)

---

## Public API - Vouch

### Get Execution Config

Main endpoint used by Vouch to fetch execution configuration.

**Endpoint**: `POST /vouch/v2/execution-config/:config`

**Path Parameters**:
- `config` (required): Name of the default config to use (e.g., `main`, `testnet`)

**Query Parameters**:
- `tags` (optional): Comma-separated list of tags to include pattern-based proposer configs (e.g., `pool-1,high-value`)

**Request Body**:
```json
{
  "keys": ["0x8021...8bbe", "0xa123...def4", "0xb456...789a"]
}
```

JSON object with `keys` field containing array of validator public keys (hex strings with 0x prefix).

**Response**: `200 OK`
```json
{
  "version": 2,
  "fee_recipient": "0x1234...5678",
  "gas_limit": "30000000",
  "min_value": "0.1",
  "relays": {
    "https://relay1.example.com/": {
      "public_key": "0xac6e77...",
      "fee_recipient": "0xabcd...ef01",
      "min_value": "0.2"
    },
    "https://relay2.example.com/": {
      "public_key": "0xbd7f88..."
    }
  },
  "proposers": [
    {
      "proposer": "0x8021...8bbe",
      "fee_recipient": "0x9999...1111",
      "min_value": "0.5"
    },
    {
      "proposer": "^Pool1/.*$",
      "fee_recipient": "0x7777...2222",
      "relays": {
        "https://relay3.example.com/": {
          "public_key": "0xce8f99..."
        }
      },
      "reset_relays": true
    }
  ]
}
```

**Response Building Logic**:
1. Load default config by name from `:config` path parameter
2. Load validator-specific configs for public keys in `keys` field (if they exist)
3. Load pattern-based proposer configs matching tags from `?tags` query parameter (OR logic)
4. Build response:
   - Top-level fields from default config
   - `proposers` array containing:
     - Validator-specific entries (for known validators from request)
     - Pattern-based entries (from matched tags)

**Error Responses**:
- `400 Bad Request`: Invalid request format
  ```json
  { "error": "Invalid request body: missing 'keys' field" }
  ```
- `404 Not Found`: Default config not found
  ```json
  { "error": "Default config 'unknown' not found" }
  ```
- `500 Internal Server Error`: Server error
  ```json
  { "error": "Database connection failed" }
  ```

**Examples**:

```bash
# Simple request with default config only
curl -X POST "http://localhost:8080/vouch/v2/execution-config/main" \
  -H "Content-Type: application/json" \
  -d '{"keys": ["0x8021...8bbe", "0xa123...def4"]}'

# Request with tags for pattern-based proposers
curl -X POST "http://localhost:8080/vouch/v2/execution-config/main?tags=pool-1,high-value" \
  -H "Content-Type: application/json" \
  -d '{"keys": ["0x8021...8bbe"]}'
```

---

## Public API - Commit-Boost

### Get Mux Keys

Main endpoint used by Commit-Boost to fetch validator keys for multiplexer configuration.

**Endpoint**: `GET /commit-boost/v1/mux/:name`

**Path Parameters**:
- `name` (required): Name of the mux configuration (e.g., `pool-1`, `mainnet-validators`)

**Response**: `200 OK`
```json
[
  "0x8160998addda06f2956e5d1945461f33dbc140486e972b96f341ebf2bdb553a0e3feb127451f5332dd9e33469d37ca67",
  "0x87b5dc7f78b68a7b5e7f2e8b9c2115f968332cbf6fc2caaaaa2c9dc219a58206b72c924805f2278c58b55790a2c3bf17",
  "0x89e2f50fe5cd07ed2ff0a01340b2f717aa65cced6d89a79fdecc1e924be5f4bbe75c11598bb9a53d307bb39b8223bc52"
]
```

Simple JSON array of validator public keys (hex strings with 0x prefix).

**Error Responses**:
- `404 Not Found`: Mux config not found
  ```json
  { "error": "Mux config 'unknown' not found" }
  ```
- `500 Internal Server Error`: Server error
  ```json
  { "error": "Database connection failed" }
  ```

**Example**:

```bash
curl -X GET "http://localhost:8080/commit-boost/v1/mux/pool-1"
```

---

## Protected API (Admin) - Vouch

All admin endpoints require authentication (TBD).

### Proposers

Proposer-specific configurations for individual validator public keys.

#### List Proposers

**Endpoint**: `GET /api/admin/vouch/proposers`

**Query Parameters**:
- `public_key` (optional): Filter by public key (exact match or prefix)
- `fee_recipient` (optional): Filter by fee recipient address
- `gas_limit` (optional): Filter by gas limit value
- `min_value` (optional): Filter by minimum value
- `reset_relays` (optional): Filter by reset_relays flag (true/false)
- `limit` (optional): Number of results per page (default: 100)
- `offset` (optional): Pagination offset (default: 0)

**Response**: `200 OK`
```json
{
  "proposers": [
    {
      "public_key": "0x8021...8bbe",
      "fee_recipient": "0x9999...1111",
      "gas_limit": null,
      "min_value": "0.5",
      "reset_relays": false,
      "created_at": "2025-01-09T10:00:00Z",
      "updated_at": "2025-01-09T10:00:00Z"
    }
  ],
  "total": 150,
  "limit": 100,
  "offset": 0
}
```

**Filter Examples**:
```bash
# Filter by fee recipient
GET /api/admin/vouch/proposers?fee_recipient=0x9999...1111

# Filter by min_value and reset_relays
GET /api/admin/vouch/proposers?min_value=0.5&reset_relays=true

# Filter by public key prefix with pagination
GET /api/admin/vouch/proposers?public_key=0x80&limit=50&offset=0
```

#### Get Proposer

**Endpoint**: `GET /api/admin/vouch/proposers/:public_key`

**Response**: `200 OK`
```json
{
  "public_key": "0x8021...8bbe",
  "fee_recipient": "0x9999...1111",
  "gas_limit": null,
  "min_value": "0.5",
  "reset_relays": false,
  "relays": [
    {
      "id": 5,
      "url": "https://relay2.example.com/",
      "public_key": "0xbd7f88...",
      "fee_recipient": null,
      "gas_limit": null,
      "min_value": null,
      "disabled": false
    }
  ],
  "created_at": "2025-01-09T10:00:00Z",
  "updated_at": "2025-01-09T10:00:00Z"
}
```

#### Create/Update Proposer

**Endpoint**: `PUT /api/admin/vouch/proposers/:public_key`

**Request Body**:
```json
{
  "fee_recipient": "0x9999...1111",
  "gas_limit": null,
  "min_value": "0.5",
  "reset_relays": false,
  "relays": [
    {
      "url": "https://relay2.example.com/",
      "public_key": "0xbd7f88...",
      "disabled": false
    }
  ]
}
```

**Response**: `200 OK` (updated) or `201 Created` (new)

#### Delete Proposer

**Endpoint**: `DELETE /api/admin/vouch/proposers/:public_key`

**Response**: `204 No Content`

---

### Default Configs

#### List Default Configs

**Endpoint**: `GET /api/admin/vouch/configs/default`

**Query Parameters**:
- `name` (optional): Filter by config name (exact match or prefix)
- `fee_recipient` (optional): Filter by fee recipient address
- `gas_limit` (optional): Filter by gas limit value
- `min_value` (optional): Filter by minimum value
- `active` (optional): Filter by active status (true/false)
- `limit` (optional): Number of results per page (default: 100)
- `offset` (optional): Pagination offset (default: 0)

**Response**: `200 OK`
```json
{
  "configs": [
    {
      "name": "main",
      "fee_recipient": "0x1234...5678",
      "gas_limit": "30000000",
      "min_value": "0.1",
      "active": true,
      "created_at": "2025-01-09T10:00:00Z",
      "updated_at": "2025-01-09T10:00:00Z"
    },
    {
      "name": "testnet",
      "fee_recipient": "0x9999...0000",
      "active": true,
      "created_at": "2025-01-08T15:00:00Z",
      "updated_at": "2025-01-08T15:00:00Z"
    }
  ]
}
```

**Filter Examples**:
```bash
# Filter by active status
GET /api/admin/vouch/configs/default?active=true

# Filter by name prefix
GET /api/admin/vouch/configs/default?name=main

# Filter by fee recipient
GET /api/admin/vouch/configs/default?fee_recipient=0x1234...5678
```

#### Get Default Config

**Endpoint**: `GET /api/admin/vouch/configs/default/:name`

**Response**: `200 OK`
```json
{
  "name": "main",
  "fee_recipient": "0x1234...5678",
  "gas_limit": "30000000",
  "min_value": "0.1",
  "active": true,
  "relays": [
    {
      "id": 1,
      "url": "https://relay1.example.com/",
      "public_key": "0xac6e77...",
      "fee_recipient": "0xabcd...ef01",
      "gas_limit": null,
      "min_value": "0.2"
    }
  ],
  "created_at": "2025-01-09T10:00:00Z",
  "updated_at": "2025-01-09T10:00:00Z"
}
```

#### Create Default Config

**Endpoint**: `POST /api/admin/vouch/configs/default`

**Request Body**:
```json
{
  "name": "main",
  "fee_recipient": "0x1234...5678",
  "gas_limit": "30000000",
  "min_value": "0.1",
  "active": true,
  "relays": [
    {
      "url": "https://relay1.example.com/",
      "public_key": "0xac6e77...",
      "fee_recipient": "0xabcd...ef01",
      "min_value": "0.2"
    }
  ]
}
```

**Response**: `201 Created`

#### Update Default Config

**Endpoint**: `PUT /api/admin/vouch/configs/default/:name`

**Request Body**: Same as create

**Response**: `200 OK`

#### Delete Default Config

**Endpoint**: `DELETE /api/admin/vouch/configs/default/:name`

**Response**: `204 No Content`

---

### Proposer Patterns

Pattern-based proposer configurations with regex matching and tags.

#### List Proposer Patterns

**Endpoint**: `GET /api/admin/vouch/proposer-patterns`

**Query Parameters**:
- `name` (optional): Filter by pattern name (exact match or prefix)
- `pattern` (optional): Filter by regex pattern (substring match)
- `tag` (optional): Filter by tag (returns patterns that have this tag)
- `fee_recipient` (optional): Filter by fee recipient address
- `gas_limit` (optional): Filter by gas limit value
- `min_value` (optional): Filter by minimum value
- `reset_relays` (optional): Filter by reset_relays flag (true/false)
- `limit` (optional): Number of results per page (default: 100)
- `offset` (optional): Pagination offset (default: 0)

**Response**: `200 OK`
```json
{
  "configs": [
    {
      "name": "pool1-mainnet",
      "pattern": "^Pool1/.*$",
      "tags": ["pool-1", "high-value"],
      "fee_recipient": "0x7777...2222",
      "gas_limit": null,
      "min_value": "0.3",
      "reset_relays": true,
      "created_at": "2025-01-09T10:00:00Z",
      "updated_at": "2025-01-09T10:00:00Z"
    },
    {
      "name": "pool1-backup",
      "pattern": "^Pool1Backup/.*$",
      "tags": ["pool-1", "backup"],
      "fee_recipient": "0x8888...3333",
      "gas_limit": null,
      "min_value": "0.2",
      "reset_relays": false,
      "created_at": "2025-01-09T11:00:00Z",
      "updated_at": "2025-01-09T11:00:00Z"
    }
  ],
  "total": 12,
  "limit": 100,
  "offset": 0
}
```

**Filter Examples**:
```bash
# Filter by tag
GET /api/admin/vouch/proposer-patterns?tag=pool-1

# Filter by multiple criteria
GET /api/admin/vouch/proposer-patterns?tag=high-value&reset_relays=true&min_value=0.3

# Filter by pattern substring
GET /api/admin/vouch/proposer-patterns?pattern=Pool1

# Filter by name prefix
GET /api/admin/vouch/proposer-patterns?name=pool1
```

#### Get Proposer Pattern

**Endpoint**: `GET /api/admin/vouch/proposer-patterns/:name`

**Response**: `200 OK`
```json
{
  "name": "pool1-mainnet",
  "pattern": "^Pool1/.*$",
  "tags": ["pool-1", "high-value"],
  "fee_recipient": "0x7777...2222",
  "gas_limit": null,
  "min_value": "0.3",
  "reset_relays": true,
  "relays": [
    {
      "id": 10,
      "url": "https://relay3.example.com/",
      "public_key": "0xce8f99...",
      "fee_recipient": null,
      "gas_limit": null,
      "min_value": null,
      "disabled": false
    }
  ],
  "created_at": "2025-01-09T10:00:00Z",
  "updated_at": "2025-01-09T10:00:00Z"
}
```

#### Create Proposer Pattern

**Endpoint**: `POST /api/admin/vouch/proposer-patterns`

**Request Body**:
```json
{
  "name": "pool1-mainnet",
  "pattern": "^Pool1/.*$",
  "tags": ["pool-1", "high-value"],
  "fee_recipient": "0x7777...2222",
  "min_value": "0.3",
  "reset_relays": true,
  "relays": [
    {
      "url": "https://relay3.example.com/",
      "public_key": "0xce8f99..."
    }
  ]
}
```

**Response**: `201 Created`

#### Update Proposer Pattern

**Endpoint**: `PUT /api/admin/vouch/proposer-patterns/:name`

**Request Body**: Same as create

**Response**: `200 OK`

#### Delete Proposer Pattern

**Endpoint**: `DELETE /api/admin/vouch/proposer-patterns/:name`

**Response**: `204 No Content`

---

## Protected API (Admin) - Commit-Boost

All admin endpoints require authentication (TBD).

### Mux Configs

#### List Mux Configs

**Endpoint**: `GET /api/admin/commit-boost/mux`

**Query Parameters**:
- `limit` (optional): Number of results per page (default: 100)
- `offset` (optional): Pagination offset (default: 0)

**Response**: `200 OK`
```json
{
  "mux_configs": [
    {
      "name": "pool-1",
      "key_count": 150,
      "created_at": "2025-01-09T10:00:00Z",
      "updated_at": "2025-01-09T10:00:00Z"
    },
    {
      "name": "testnet",
      "key_count": 25,
      "created_at": "2025-01-08T15:00:00Z",
      "updated_at": "2025-01-08T15:00:00Z"
    }
  ],
  "total": 2,
  "limit": 100,
  "offset": 0
}
```

#### Get Mux Config

**Endpoint**: `GET /api/admin/commit-boost/mux/:name`

**Response**: `200 OK`
```json
{
  "name": "pool-1",
  "keys": [
    "0x8160998addda06f2956e5d1945461f33dbc140486e972b96f341ebf2bdb553a0e3feb127451f5332dd9e33469d37ca67",
    "0x87b5dc7f78b68a7b5e7f2e8b9c2115f968332cbf6fc2caaaaa2c9dc219a58206b72c924805f2278c58b55790a2c3bf17"
  ],
  "created_at": "2025-01-09T10:00:00Z",
  "updated_at": "2025-01-09T10:00:00Z"
}
```

#### Create Mux Config

**Endpoint**: `POST /api/admin/commit-boost/mux`

**Request Body**:
```json
{
  "name": "pool-1",
  "keys": [
    "0x8160998addda06f2956e5d1945461f33dbc140486e972b96f341ebf2bdb553a0e3feb127451f5332dd9e33469d37ca67",
    "0x87b5dc7f78b68a7b5e7f2e8b9c2115f968332cbf6fc2caaaaa2c9dc219a58206b72c924805f2278c58b55790a2c3bf17"
  ]
}
```

**Response**: `201 Created`
```json
{
  "name": "pool-1",
  "key_count": 2,
  "created_at": "2025-01-09T10:00:00Z",
  "updated_at": "2025-01-09T10:00:00Z"
}
```

#### Update Mux Config

**Endpoint**: `PUT /api/admin/commit-boost/mux/:name`

**Request Body**: Same as create

**Response**: `200 OK`

#### Delete Mux Config

**Endpoint**: `DELETE /api/admin/commit-boost/mux/:name`

**Response**: `204 No Content`

#### Add Keys to Mux

**Endpoint**: `POST /api/admin/commit-boost/mux/:name/keys`

**Request Body**:
```json
{
  "keys": [
    "0x89e2f50fe5cd07ed2ff0a01340b2f717aa65cced6d89a79fdecc1e924be5f4bbe75c11598bb9a53d307bb39b8223bc52"
  ]
}
```

**Response**: `200 OK`
```json
{
  "added": 1,
  "total_keys": 3
}
```

#### Remove Keys from Mux

**Endpoint**: `DELETE /api/admin/commit-boost/mux/:name/keys`

**Request Body**:
```json
{
  "keys": [
    "0x89e2f50fe5cd07ed2ff0a01340b2f717aa65cced6d89a79fdecc1e924be5f4bbe75c11598bb9a53d307bb39b8223bc52"
  ]
}
```

**Response**: `200 OK`
```json
{
  "removed": 1,
  "total_keys": 2
}
```

---

## Data Types

### Ethereum Address
- Format: Hex string with `0x` prefix
- Length: 42 characters (0x + 40 hex digits)
- Example: `"0x1234567890abcdef1234567890abcdef12345678"`

### Validator Public Key
- Format: Hex string with `0x` prefix
- Length: 98 characters (0x + 96 hex digits)
- Example: `"0x8021...8bbe"` (shortened for readability)

### Gas Limit
- Format: String containing numeric value
- Example: `"30000000"`

### Min Value
- Format: String containing decimal value in ETH
- Example: `"0.1"`, `"0.25"`

### Regex Pattern
- Format: String containing valid regex pattern
- Used in proposer configs for pattern-based matching
- Example: `"^Pool1/.*$"`, `"^Wallet [0-9]+/.*$"`

### Tags
- Format: Array of strings
- Used for grouping pattern-based proposer configs
- Example: `["pool-1", "high-value", "relay-A"]`

---

## Notes

1. **Field Nullability**: All config fields (`fee_recipient`, `gas_limit`, `min_value`) are optional and can be `null`. Null values mean "use parent/default value".

2. **Relay Merging**: By default, relays are merged. Set `reset_relays: true` to replace all relays instead of merging.

3. **Relay Disabled**: In validator configs, individual relays can be marked `disabled: true` to exclude them from the final config.

4. **Timestamps**: All timestamps are in ISO 8601 format with UTC timezone.

5. **Pagination**: List endpoints support `limit` and `offset` query parameters for pagination.

6. **Filtering**: Vouch list endpoints support filtering via query parameters:
   - **String fields** (name, public_key, fee_recipient, pattern): Exact match or prefix matching
   - **Numeric fields** (gas_limit, min_value): Exact match
   - **Boolean fields** (active, reset_relays): true/false values
   - **Array fields** (tags): Match if the item contains the specified tag
   - Multiple filters can be combined with AND logic
   - All filters are optional
   - Commit-Boost mux endpoints do not support filtering (simple list only)

7. **Validation**:
   - Ethereum addresses must be valid checksummed addresses
   - Validator public keys must be valid BLS public keys
   - Regex patterns must be valid regex syntax
   - Gas limits and min values must be parseable numbers

8. **Config Precedence in Response**:
   - Validator-specific config overrides default config
   - Pattern-based configs are added as separate proposer entries
   - First matching proposer in array takes precedence (Vouch behavior)
