# Design

## Context

- Handlers use Axum's `Json`, `Query` and `Path` directly. Their rejections are plain-text responses (`400`, `415` or `422` for JSON; `400` for query and path) produced before our code runs, so `ApiError` never sees them.
- `ApiError` (`src/errors.rs`) maps `RowNotFound` to `404` and every other database error to `500` with code `DATABASE_ERROR`. Duplicates are detected by a `SELECT COUNT(*)` pre-check that returns `InvalidData` (`400`); a concurrent insert slips past it and hits the primary key as a `500`.
- `UpdateDefaultConfigRequest` and `UpdateProposerPatternRequest` are all-`Option` structs applied with `COALESCE`. Relays are replaced only when `relays` is present. Proposers (`CreateOrUpdateProposerRequest`) and mux configs already replace in full.
- Proposer handlers take `Path<String>` and bind it as-is. `BlsPubkey` (`src/addresses/bls.rs`) parses either hex case and displays lower case, and its `Encode` writes the lower-case form, which is how request keys reach the execution-config query.
- List handlers accept any `i64` for `limit`/`offset`, run a `COUNT(*)` per page, and return `PaginatedResponse { data, total, limit, offset }`. Proposers sort by `created_at DESC` alone; the other lists sort by their primary key `name`. Measured with 100,000 proposers: the last offset page takes 54 ms end to end, so this is a correctness and consistency change, not a performance fix.
- No route sets a body limit, so Axum's default 2 MB applies everywhere. A 100,000-key execution-config request (10.2 MB) gets a plain-text `413`; the cut-off is about 20,700 keys. Kleido (`~/src/p2p/kleido`) uses keyset pagination and a 10 MB limit on its POST routes, and is the reference for both.

## Goals / Non-Goals

**Goals:**
- No response leaves the service with a non-JSON error body, and every input error is `400`.
- Small, mechanical changes per handler: swap extractors, drop the hand-written duplicate checks' status, add bounds checks.

**Non-Goals:**
- Pagination for `GET /api/admin/tokens`. The token list is tiny and unpaginated today; out of scope.
- `PATCH`. Rejected in favour of a single full-replacement `PUT` (decided with the user).
- Rewriting non-canonical proposer keys already stored. They are found by a query and fixed by hand (see Migration Plan).
- Raising the body limit on admin routes, and other transport-level errors. Only the over-limit rejection gets the JSON body.
- A `total` count in list responses. Kleido does without one, and keeping it would cost a `COUNT(*)` per page.

## Decisions

### Wrapper extractors with `ApiError` as the rejection
Add `src/extract.rs` with `AppJson<T>`, `AppQuery<T>` and `AppPath<T>`. Each is a newtype deriving `FromRequest`/`FromRequestParts` via Axum's own extractor with `rejection(ApiError)`, which needs Axum's `macros` feature. Add `From<JsonRejection>`, `From<QueryRejection>` and `From<PathRejection>` for `ApiError`:

- `JsonRejection::JsonSyntaxError` and `MissingJsonContentType` → `InvalidJson` (`400 INVALID_JSON`);
- `JsonRejection::BytesRejection` whose status is `413` (body over the limit) → `PayloadTooLarge` (`413 PAYLOAD_TOO_LARGE`);
- `JsonDataError` → `InvalidData` (`400 INVALID_DATA`), keeping serde's message, which names the field path;
- any `QueryRejection` → `InvalidQuery` (`400 INVALID_QUERY`);
- any `PathRejection` → `InvalidData`.

Every handler swaps `Json(x)` → `AppJson(x)` and so on, `AppJson` included on the public execution-config endpoint. Response bodies keep using `axum::Json`; only extraction changes.

*Alternative considered*: a middleware that rewrites plain-text `4xx` responses into JSON. It works without touching handlers, but it has to sniff bodies and guess the cause, and it loses serde's field path. Wrapper extractors are the pattern Axum documents for this.

### New `ApiError` variants and codes
Add `InvalidJson`, `InvalidQuery`, `Conflict(String)` (`409 CONFLICT`), `PayloadTooLarge` (`413 PAYLOAD_TOO_LARGE`) and `MethodNotAllowed` (`405 METHOD_NOT_ALLOWED`). Database errors other than `RowNotFound` and unique violations become `500` with code `INTERNAL_ERROR` instead of `DATABASE_ERROR`, matching the spec's single code for unexpected failures. A unique violation (`23505`, read via `sqlx::Error::as_database_error().code()`) maps to `409 CONFLICT` with a generic message.

### Duplicates: keep the friendly pre-check, rely on the constraint for races
The existing `SELECT COUNT(*)` checks in the create handlers for default configs, patterns and mux configs now return `ApiError::Conflict("… '<name>' already exists")`. When two creates race, the pre-check passes for both, and the loser's `INSERT` hits the primary key; the `23505` mapping above turns that into `409` too. That meets the spec without restructuring the handlers around `INSERT … ON CONFLICT`.

### Full-replacement PUT
`UpdateDefaultConfigRequest` becomes the create request minus `name`: optional `fee_recipient`/`gas_limit`/`min_value`, `active` defaulting to `true`, optional `relays`. `UpdateProposerPatternRequest` becomes the create request minus `name`, with `pattern: String` required and `tags` defaulting to empty. The update handlers:
1. 404 if the resource does not exist;
2. `UPDATE … SET` every column from the request, no `COALESCE`;
3. delete all relays, then insert the request's relays, if any;

all in the existing transaction. Validation (`src/validation.rs`) adapts: `pattern` is no longer optional on update.

### Proposer path key via `AppPath<BlsPubkey>`
Proposer handlers take `AppPath<BlsPubkey>`. Parsing rejects invalid keys (`400 INVALID_DATA`). Binding a `BlsPubkey` writes its lower-case form, so `PUT`, `GET` and `DELETE` all address the canonical row, the same one the execution-config query finds. Responses and audit entries use its `Display` form.

### Keyset pagination, as in kleido
- **Shared request fields.** Each `*Filters` struct carries `limit: i64` (default 100 via `pagination::default_limit`) and `after: Option<String>` directly, and `pagination::check_limit` returns `InvalidQuery` outside `1..=1000`. *Changed during implementation*: the plan was a shared struct flattened into each filter struct with `DisplayFromStr`, as in kleido. Declaring the two fields in place avoids `flatten`'s string-only quirk entirely and keeps `IntoParams` documenting them, with the same behaviour.
- **Query.** Each list's existing `QueryBuilder` gains `AND <pk> > $after` when `after` is present, then `ORDER BY <pk> LIMIT $limit`, where `<pk>` is `p.public_key` for proposers and `name` otherwise. The primary-key index serves both the filter and the order. The per-page `COUNT(*)` query is removed.
- **Response.** A generic `ListResponse<T> { items: Vec<T>, next_cursor: Option<String> }`, with `next_cursor` skipped when `None`, replaces `PaginatedResponse`.
- **`next_cursor` rule.** When the page is full (`items.len() == limit`), `next_cursor` is the last item's key. This is kleido's rule: a list whose size is an exact multiple of `limit` ends with one empty page, an accepted cost of not querying `limit + 1`.
- **Cursor format.** The cursor is the raw key string. It is documented as opaque so the format can change, but it is not encrypted or signed: it only selects a position in data the caller can already list.

*Alternative considered*: keep offset paging and add a tie-breaker. Cheaper to change, but walks still repeat or skip entries when rows are added mid-walk, and kleido already sets the team convention.

### Body limit on the public Vouch router
`vouch::public_routes()` gets `.layer(DefaultBodyLimit::max(10 * 1024 * 1024))`. The layer sits on that sub-router only, so admin routes keep the 2 MB default. 10 MB is about 100,000 keys, roughly three times the expected production maximum of 30,000. The resulting rejection reaches the handler's `AppJson` as a `BytesRejection` with status `413`, which maps to `PayloadTooLarge`.

### Router fallbacks
`Router::fallback` returns `ApiError::NotFound("route not found")` and `Router::method_not_allowed_fallback` returns `ApiError::MethodNotAllowed`, both set on the top-level router so they cover the public routes, the admin routes and the Swagger paths. The auth middleware stays on the admin sub-router, so an unknown path under `/api/admin` without a token returns `401` before the fallback runs. That is acceptable and does not leak route existence.

### OpenAPI error responses via a modifier
Rather than hand-editing ~25 `#[utoipa::path]` annotations, add a `Modify` in `src/openapi.rs` that walks every operation and adds the `ErrorResponse` body to:
- `400` and `500` on every operation;
- `401` on `/api/admin/*`;
- `404` on paths with a path parameter;
- `409` on `POST` to a collection.

Responses already declared on an operation, such as the `400` added by `harden-execution-config`, are left as they are. `openapi.json` is regenerated.

## Risks / Trade-offs

- [Admin scripts that send a partial `PUT` silently clear fields] → The most dangerous part of the change. Called out as BREAKING in the proposal, the README and the PR description. A careful client sends a GET-modify-PUT. Nothing in this repository relies on partial `PUT`, and the existing tests that did are updated in the tasks.
- [Clients matching on `422` or `DATABASE_ERROR`] → Breaking but rare; documented.
- [Clients paging with `offset`, or reading `total`/`data`] → They break. Called out as BREAKING; the README documents the cursor walk.
- [Proposer list no longer shows newest first] → Admins who want recent changes sort client-side or filter; ordering by key is what makes the walk stable.
- [A 10 MB request is held in memory while it is parsed] → At most about 10 MB per in-flight Vouch request; Vouch instances are few.
- [Unknown `/api/admin/*` paths return `401` without a token] → Intended; the fallback answers `404` only for authenticated requests or non-admin paths.

## Migration Plan

1. Before deploying, run the extended `scripts/check_invalid_config.sql`. It now also lists proposer keys that are not lower-case BLS keys. Re-create any it finds under the lower-case key, then delete the old row. The old row can only be deleted with a direct SQL `DELETE`, because the API now normalizes the key it is given.
2. Tell admin API users about full-replacement `PUT` and the new status codes before the deploy.
3. Deploy. No schema migration. Rollback is redeploying the previous image.
