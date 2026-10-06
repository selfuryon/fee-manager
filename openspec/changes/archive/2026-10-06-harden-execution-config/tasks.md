# Tasks

## 1. Pre-deploy data check

- [x] 1.1 Write `scripts/check_invalid_config.sql`, a read-only query that lists every stored `gas_limit`, `min_value`, relay URL (all three relay tables) and proposer `pattern` that the new rules would reject. Approximate the rules with Postgres regexes; patterns are only checked for being non-empty, since only the service can compile them. Verify by running it against the local database seeded with `scripts/seed_testdata.sql`: it returns no rows. Inserting one bad row by hand makes it return exactly that row.

## 2. Validation module

- [x] 2.1 Add the `regex` crate to `Cargo.toml` and verify `cargo build --locked` succeeds after updating `Cargo.lock`.
- [x] 2.2 Create `src/validation.rs` with `gas_limit`, `min_value`, `relay_url` and `pattern` checks per design.md "Validation rules", returning `ApiError::InvalidData` with the field name in the message. Verify with unit tests in the module covering each spec scenario value: `"30000000"`, `"0"`, `"lots"`, `" 1"`, `"+1"`, `"1e3"` for gas; `"0"`, `"0.10"`, `"-1"`, `"cheap"`, `"1e-3"` for min value; `https://relay.example.invalid/`, `relay.example.invalid`, `ftp://relay.example.invalid/` for URLs; `^pool/.*$`, `([unclosed` for patterns.
- [x] 2.3 Add per-request `validate()` helpers for `CreateDefaultConfigRequest`, `UpdateDefaultConfigRequest`, `CreateOrUpdateProposerRequest`, `CreateProposerPatternRequest` and `UpdateProposerPatternRequest`. They walk every present field and every relay entry, prefixing relay errors with the relay URL. Verify with unit tests: a relay error message contains both the field and the URL, and `None` fields pass.

## 3. Wire validation into admin handlers

- [x] 3.1 Call `validate()` first in the create and update handlers of `src/handlers/vouch/default_configs.rs`, before the transaction opens, and add a `400` response to their utoipa annotations. Verify with integration tests in `tests/default_configs_test.rs`: invalid `gas_limit`, `min_value`, relay URL and relay-nested values return `400` `INVALID_DATA`, nothing is stored, and an update with one bad relay leaves the existing relays unchanged.
- [x] 3.2 Do the same in `src/handlers/vouch/proposers.rs`. Verify with integration tests in `tests/proposers_test.rs`: `gas_limit` `"0"` on update returns `400` and the proposer is unchanged.
- [x] 3.3 Do the same in `src/handlers/vouch/proposer_patterns.rs`, including `pattern` on create and update. Verify with integration tests in `tests/proposer_patterns_test.rs`: `"([unclosed"` returns `400`, and `min_value` `"0.10"` round-trips unchanged.
- [x] 3.4 Regenerate the API contract with `UPDATE_OPENAPI=1 cargo test --test openapi_test` and verify `openapi.json` shows the new `400` responses on the affected admin operations.
- [x] 3.5 Document the accepted value formats in the README "Execution config semantics" section and verify the README examples still use valid values.

## 4. Execution config: snapshot, batching, order

- [x] 4.1 In `src/handlers/vouch/execution_config.rs`, run all reads in one transaction that starts with `SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY`. Verify the existing `tests/execution_config_test.rs` still passes.
- [x] 4.2 Load proposers `ORDER BY public_key`, and their relays with a single `= ANY($1)` query grouped in memory. Remove the per-proposer query from the loop. Verify with a new integration test that requests 200 known keys with relays, expects all 200 entries in public-key order with correct relays, and finishes in under a second locally.
- [x] 4.3 Load patterns `ORDER BY name`, stable-sort them by first-tag position, and load their relays with a single `= ANY($1)` query. Verify with a new integration test: patterns `zeta` and `alpha` sharing one requested tag come back `alpha` first; the existing tag-order test still passes.
- [x] 4.4 Switch `relays` in `ExecutionConfigResponse` and `ProposerEntry` (`src/schema.rs`) to `BTreeMap`. Verify with a new integration test that two identical requests return byte-identical bodies, and that `UPDATE_OPENAPI=1 cargo test --test openapi_test` produces no `openapi.json` diff for these schemas.

## 5. Integration checks

- [x] 5.1 Run `cargo fmt --all --check`, `SQLX_OFFLINE=true cargo clippy --locked --all-targets -- -D warnings` and `cargo test --locked --all-targets`, and verify all pass.
- [x] 5.2 Run `cargo sqlx prepare -- --all-targets` and verify `.sqlx/` has no diff, or commit the diff if a compile-time-checked query changed.
- [x] 5.3 Run `cargo test --test openapi_test` without `UPDATE_OPENAPI` and verify it passes against the committed `openapi.json`.
