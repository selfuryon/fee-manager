# Tasks

## 1. Pre-deploy data check

- [x] 1.1 Extend `scripts/check_invalid_config.sql` to also list `vouch_proposers.public_key` and `vouch_proposer_relays.proposer_public_key` values that do not match `^0x[0-9a-f]{96}$`. Verify against the local database: no rows on seeded data; a proposer inserted by hand with an upper-case key is listed.

## 2. Error model and extractors

- [x] 2.1 Enable Axum's `macros` feature and verify `cargo build --locked` succeeds after updating `Cargo.lock`.
- [x] 2.2 In `src/errors.rs`, add `InvalidJson`, `InvalidQuery`, `Conflict(String)`, `PayloadTooLarge` and `MethodNotAllowed`; map unique violations (`23505`) to `409 CONFLICT`; rename the generic database `500` code to `INTERNAL_ERROR`; add `From` impls for `JsonRejection`, `QueryRejection` and `PathRejection` per design.md. Verify with unit tests on the `IntoResponse` mapping: status, `code`, and that a database error's text is not in the message.
- [x] 2.3 Create `src/extract.rs` with `AppJson`, `AppQuery` and `AppPath` using `ApiError` as the rejection, then replace `Json`/`Query`/`Path` extraction in every handler, including `src/auth/handlers.rs` and the public endpoints. Verify `grep -rn 'Json(\w*): Json<\|Query<\|Path<' src/handlers src/auth` finds no Axum extractors left, and the existing test suite passes.
- [x] 2.4 Add `fallback` and `method_not_allowed_fallback` to the top-level router in `src/handlers/mod.rs`. Verify with integration tests in a new `tests/errors_test.rs`:
  - malformed JSON → `400 INVALID_JSON`, with a JSON content type;
  - `"active": "yes"` → `400 INVALID_DATA` naming `active`;
  - `["0x12"]` to the execution-config endpoint → `400 INVALID_DATA`;
  - `?limit=abc` → `400 INVALID_QUERY`;
  - `DELETE /api/admin/tokens/not-a-uuid` → `400 INVALID_DATA`;
  - an unknown route with a token → `404 NOT_FOUND`;
  - `PATCH /api/admin/vouch/proposers` → `405 METHOD_NOT_ALLOWED`;
  - no token → `401 UNAUTHORIZED`.

## 3. Conflicts

- [x] 3.1 Make the duplicate pre-checks in the create handlers for default configs, proposer patterns and mux configs return `ApiError::Conflict`. Update the existing duplicate tests (`tests/default_configs_test.rs`, `tests/proposer_patterns_test.rs`, `tests/mux_test.rs`) from `400` to `409` and assert `error.code` `CONFLICT`. Verify they pass.
- [x] 3.2 Add an integration test that fires two concurrent creates of the same proposer pattern name and expects exactly one `201` and one `409`. Verify it passes repeatedly (`cargo test … -- --test-threads=1`, run 5×).

## 4. Keyset pagination

- [x] 4.1 Add `Pagination` (`limit` via `DisplayFromStr`, default 100, checked to 1–1000; optional `after`) and a generic `ListResponse<T> { items, next_cursor }` per design.md, replacing `PaginatedResponse`. Verify with unit tests: `check()` rejects 0 and 1001 and accepts 1 and 1000, and `next_cursor` is skipped when `None`.
- [x] 4.2 Switch the four list handlers (proposers, default configs, proposer patterns, mux configs) to `AND <pk> > $after ORDER BY <pk> LIMIT $limit`, drop their `COUNT(*)`, and set `next_cursor` to the last key when the page is full. Verify with integration tests:
  - `limit=0` and `limit=1001` → `400 INVALID_QUERY`, `limit=1000` → `200`;
  - three configs walked with `limit=2`: two items plus a cursor, then one item and no cursor;
  - a filtered walk returns exactly the matching items;
  - walking proposers with `limit=1` while a proposer with a lower key is inserted mid-walk returns each pre-existing proposer exactly once.
- [x] 4.3 Update the existing pagination and list tests (`test_page_*` and other `data`/`total` readers in `tests/`) to the `items`/`next_cursor` shape, and verify they pass.

## 5. Execution-config body limit

- [x] 5.1 Add `DefaultBodyLimit::max(10 * 1024 * 1024)` to `vouch::public_routes()` and map the over-limit `BytesRejection` to `PayloadTooLarge`. Verify with integration tests:
  - a 30,000-key request (~3 MB) → `200`;
  - a body over 10 MB → `413` with JSON `error.code` `PAYLOAD_TOO_LARGE`;
  - a 3 MB body to `POST /api/admin/vouch/configs/default` → still `413` (admin limit unchanged), now with the JSON body.

## 6. Full-replacement PUT and proposer key identity

- [x] 6.1 Reshape `UpdateDefaultConfigRequest` and `UpdateProposerPatternRequest` in `src/schema.rs` per design.md (`pattern` required, `tags`/`active` defaults), and adapt their `validate()` in `src/validation.rs`. Verify the unit tests in `src/validation.rs` pass after updating them.
- [x] 6.2 Rewrite `update_default_config` and `update_proposer_pattern` to set every column from the request and replace relays unconditionally. Verify with integration tests covering the `vouch/config-management` scenarios: omitted fields cleared, relays and tags emptied, missing `pattern` → `400` with the pattern unchanged, `PUT` to a missing name → `404`. Update `test_update_default_config`, `test_update_proposer_pattern` and `test_update_with_invalid_relay_keeps_existing_relays` to the new semantics.
- [x] 6.3 Switch the proposer handlers to `AppPath<BlsPubkey>`. Verify with integration tests:
  - a proposer created with an upper-case path key is served to a lower-case execution-config request;
  - `GET` with the other case returns `200` and a lower-case `public_key`;
  - `PUT /api/admin/vouch/proposers/not-a-key` → `400 INVALID_DATA`.

## 7. Documentation and contract

- [x] 7.1 Replace the `PaginatedResponse<…>` schema registrations in `src/openapi.rs` with `ListResponse<…>`, add the error-response modifier per design.md, and regenerate with `UPDATE_OPENAPI=1 cargo test --test openapi_test`. Verify in `openapi.json` that every operation has `400` and `500`, admin operations `401`, parameterized paths `404` and collection `POST`s `409`, each referencing `ErrorResponse`.
- [x] 7.2 Document in the README the error body and codes (including `413`), cursor pagination with a walk example, the 10 MB execution-config limit, full-replacement `PUT` (with a warning about partial updates) and proposer key normalization. Update `CLAUDE.md` if any of its guidance contradicts the new behaviour. Verify by reading the affected sections against the specs.

## 8. Integration checks

- [x] 8.1 Run `cargo fmt --all --check`, `SQLX_OFFLINE=true cargo clippy --locked --all-targets -- -D warnings` and `cargo test --locked --all-targets`, and verify all pass.
- [x] 8.2 Run `cargo sqlx prepare -- --all-targets`; verify `.sqlx/` has no diff, or commit the diff if a compile-time-checked query changed.
- [x] 8.3 Run `cargo test --test openapi_test` without `UPDATE_OPENAPI` and verify it passes against the committed `openapi.json`.
