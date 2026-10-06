# Design

## Context

- Request and response types share structs. `RelayConfig` and `ProposerRelayConfig` in `src/schema.rs` appear in create/update requests and in responses alike, and their `gas_limit`/`min_value` are `Option<String>`. The database columns are `TEXT`.
- Public keys and fee recipients are already validated at deserialization by the `BlsPubkey`/`EthAddress` newtypes. A failure there surfaces as Axum's plain-text `422` rejection, not the service's JSON error body; unifying that is the next change (`consistent-admin-api`), not this one.
- Write handlers open a transaction after deserializing the request, then check existence and insert. There is no validation step.
- `get_execution_config` runs one `SELECT` for the default config, one for its relays and one for the proposers, then one relay `SELECT` per proposer and per pattern, all on the pool without a transaction. Pattern order comes from a stable `sort_by_key` over rows in unspecified order, and relay maps are `HashMap`s.

## Goals / Non-Goals

**Goals:**
- One validation module that every admin write path calls before touching the database, returning the existing `ApiError::InvalidData` (400 with `INVALID_DATA`).
- The execution-config handler issues a fixed number of queries inside one read-only transaction and produces byte-identical output for identical input.

**Non-Goals:**
- Re-validating or rewriting stored data, or refusing to serve it. A stored value that would fail validation keeps being served; finding such rows is a one-off check, see the Migration Plan.
- Changing how key and address validation errors look (still Axum's `422`), or how duplicates are reported. Both belong to `consistent-admin-api`.
- Checking that a regex is meaningful for Vouch (for example that it can ever match an account name). We only guarantee that it compiles.

## Decisions

### Validate in handlers, not by changing field types
Add `src/validation.rs` with small functions (`gas_limit`, `min_value`, `relay_url`, `pattern`) and one `validate()` per request type that walks every field, including each relay, and returns the first failure as `ApiError::InvalidData("<field>: <reason>")`, prefixed with the relay URL for relay fields. Each create/update handler calls it first, before `pool.begin()`.

*Alternative considered*: validating newtypes (`GasLimit`, `MinValue`, `RelayUrl`), the way `BlsPubkey` works. That gives type-level guarantees, but those types would have to live in the shared request/response structs, so reading a row that holds an old invalid value would fail to decode and turn the public endpoint into a `500`. Avoiding that means splitting request and response types, which is a larger refactor than this change. The newtypes would also report failures as plain-text `422`, while the spec wants the JSON `400` body. Handler validation keeps every type and the stored text as they are.

### Validation rules
- `gas_limit`: ASCII digits only, parsed as `u64`, greater than 0. Rejecting `+`, whitespace and leading or trailing garbage keeps what is stored identical to what Vouch parses.
- `min_value`: must have the form `digits` or `digits.digits` (so no sign, exponent, whitespace, or bare `.5` / `1.`), then parse with `rust_decimal::Decimal::from_str`. The result must be `>= 0`. Precision is not capped: Vouch converts ETH to wei itself.
- Relay URL: `url::Url::parse` must succeed, the scheme must be `http` or `https`, and the URL must have a host. The stored key stays the original string, since Vouch keys relays by the exact URL and a trailing slash matters.
- Pattern: `regex::Regex::new` must succeed. Rust's `regex` crate follows RE2 syntax, the same family as Go's `regexp` used by Vouch, so constructs either side rejects (backreferences, lookaround) are rejected here too. The two are not identical (Unicode class details, a size limit), which is acceptable: the check exists to stop typos, not to prove equivalence.

### Execution config: batched queries in one read-only snapshot
Open `pool.begin()` and run `SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY` as the first statement, so every query sees the same snapshot. Then:
1. Default config (`active = true`), as now.
2. Default relays.
3. Proposers `WHERE public_key = ANY($1) ORDER BY public_key`.
4. Relays for all of them: `WHERE proposer_public_key = ANY($1)`, grouped in memory by key.
5. Patterns `WHERE tags && $1 ORDER BY name`, then a stable sort by first-tag position, so ties keep name order.
6. Relays for those patterns: `WHERE pattern_name = ANY($1)`, grouped in memory.

That is at most 6 queries for any request, compared with 3 + N + M today. Steps 4 and 6 are skipped when there is nothing to look up.

*Alternative considered*: one query with `json_agg` joins. Fewer round trips, but the result would no longer map onto the existing `FromRow` models, and the query would be much harder to read. Six fixed queries already meet the spec.

### Stable relay map order
Switch `relays` in `ExecutionConfigResponse` and `ProposerEntry` from `HashMap` to `BTreeMap`. serde serializes it in key order, and utoipa describes it as the same JSON object, so `openapi.json` does not change shape. The admin response types keep `HashMap`; nothing requires order there.

### Error messages stay the existing body
Validation errors reuse `ApiError::InvalidData`, so admin clients see the error format they already handle. The utoipa annotations of the affected admin handlers gain a `400` response entry, and `openapi.json` is regenerated.

## Risks / Trade-offs

- [Rust `regex` and Go RE2 disagree on an edge-case pattern] → Rare: both use the RE2 syntax family, and the failure modes are a false reject (the admin rewrites the pattern) or a false accept (no worse than today).
- [An admin integration relied on sending loose values such as `" 30000000"`] → Gets `400` with the field named, a one-line fix on their side. Called out as BREAKING (admin API only) in the proposal.
- [Old invalid rows keep being served] → Deliberate, so as not to break Vouch on deploy. The Migration Plan finds them before rollout.
- [REPEATABLE READ might raise serialization errors] → Read-only transactions under REPEATABLE READ in Postgres never fail with serialization errors; they only read the snapshot.

## Migration Plan

1. Before deploying, run the read-only check query that is part of the tasks against production to list stored rows whose `gas_limit`, `min_value`, relay URL or pattern would now fail validation. Fix any it finds through the admin API, or deliberately leave them, since they keep being served.
2. Deploy. No schema migration runs.
3. Rollback is redeploying the previous image. No data was changed, so nothing needs undoing.
