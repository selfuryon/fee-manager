# Proposal

## Why

The execution config is the only thing Vouch reads from this service, and today it can be wrong in two ways. The admin API stores any string as `gas_limit`, `min_value`, a relay URL or a proposer pattern, and the public endpoint serves it to Vouch as is: `gas_limit: "lots"` and an unclosed regex were both accepted with `201` and served from `POST /vouch/v2/execution-config/{config}`. Separately, pattern entries that tie on tag position come back in whatever order Postgres returns them, and since Vouch applies the first matching entry, a validator matched by two such patterns can get a different fee recipient from one request to the next. The endpoint also issues one relay query per known proposer and per pattern, so its cost grows with the number of keys a Vouch instance sends.

## What Changes

- The admin API rejects invalid values on create and update of default configs, proposers and proposer patterns, including the relays nested in each. The response is `400` with the existing `INVALID_DATA` error body, which names the offending field:
  - `gas_limit`: a positive integer, written as a decimal string.
  - `min_value`: a non-negative decimal number of ETH, written as a string.
  - relay URL (the key of a `relays` map): an absolute `http` or `https` URL.
  - proposer `pattern`: a regular expression that compiles.
- **BREAKING (admin API only)**: requests that were accepted before with such values now fail with `400`. Valid requests behave exactly as before, and values are stored exactly as sent, not normalized.
- The execution-config response orders its entries deterministically: proposer entries by public key; pattern entries by the position of their first matching tag in `?tags`, then by pattern name. Relay maps are serialized in a stable key order.
- The execution-config endpoint reads everything from one consistent snapshot and loads relays in a fixed number of queries, however many keys are requested.

**Effect on Vouch and Commit-Boost**: the shape of the execution-config response does not change. Vouch receives the same values. The only differences are that the order of pattern entries that tied on tag position is now fixed (by pattern name), and relay maps serialize in a fixed key order, which carries no meaning in JSON. Commit-Boost is not affected.

**Database migration**: none. Columns stay `TEXT`. Values already stored are not re-validated or rewritten; they are still served as they are, and the implementation includes a one-off query to find any that would now fail validation.

## Capabilities

### New Capabilities
- `vouch/config-validation`: what the admin API accepts as a gas limit, minimum value, relay URL and proposer pattern, and how it rejects anything else.
- `vouch/execution-config`: how the public execution-config response is assembled: which entries it contains, in what order, and that it reflects a single consistent state of the configuration.

### Modified Capabilities

None. `openspec/specs/` is empty; this change introduces the first specs.

## Impact

- **Code**: a new validation module used by the create/update handlers in `src/handlers/vouch/{default_configs,proposers,proposer_patterns}.rs`; `src/handlers/vouch/execution_config.rs` rewritten around batched relay queries in a read-only transaction; relay maps in `src/schema.rs` response types switch to an ordered map.
- **Dependencies**: adds the `regex` crate. `url` and `rust_decimal` are already dependencies.
- **API**: admin endpoints document the `400` response for invalid values; `openapi.json` is regenerated. Public endpoints keep their paths, request and response shape.
- **Data**: no migration; existing invalid rows, if any, are found by a query and fixed through the admin API.
