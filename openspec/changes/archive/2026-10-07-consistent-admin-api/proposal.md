# Proposal

## Why

The admin API's behaviour depends on which layer catches a mistake. Malformed JSON gets a plain-text `400`, an invalid key or address a plain-text `422`, our own validation a JSON `400`, and a duplicate name `400` even though the OpenAPI document promises `409`; two concurrent creates of the same name get `500`. `?offset=-1` is a `500`, `limit` has no ceiling, and offset paging repeats or skips entries when rows are added mid-walk. The public execution-config endpoint keeps Axum's default 2 MB body limit, which a Vouch instance with more than about 20,700 validators exceeds (measured: 100,000 keys get a plain-text `413`); production instances are expected to reach 30,000. Updates of default configs and proposer patterns are partial, so a field, once set, can never be cleared, while proposers and mux configs are replaced in full. And the proposer path key is stored without parsing: a key sent in upper case is stored that way and never matches the lower-case keys Vouch sends, so that proposer's config is silently never served.

## What Changes

- **One error format.** Every error response carries the JSON body `{"error": {"code", "message"}}`, including rejections of malformed JSON, wrong field types, invalid keys and addresses, bad query parameters and bad path parameters, and unknown routes and methods.
- **BREAKING: invalid input is always `400`.** Cases that returned `422` (invalid key or address in a body) now return `400`. The `code` distinguishes the cause: `INVALID_JSON`, `INVALID_DATA`, `INVALID_QUERY`.
- **BREAKING: duplicates are `409`.** Creating a default config, proposer pattern or mux config whose name exists returns `409` `CONFLICT`, including when two creates race.
- **BREAKING: keyset pagination.** The four admin list endpoints switch from `limit`/`offset` to `limit` (1–1000, default 100) plus an opaque `after` cursor, the same contract kleido uses. The response becomes `{"items": [...], "next_cursor": "..."}`, with `next_cursor` omitted on the last page, replacing `{data, total, limit, offset}`. Items are ordered by primary key, so proposers are ordered by public key instead of newest first. An out-of-range `limit` is `400` `INVALID_QUERY`.
- **Larger execution-config requests.** `POST /vouch/v2/execution-config/{config}` accepts bodies up to 10 MB (about 100,000 keys). A larger body gets `413` `PAYLOAD_TOO_LARGE` with the JSON error body. Admin routes keep the default limit.
- **BREAKING: `PUT` replaces.** `PUT` on a default config or a proposer pattern replaces the whole resource, the way proposers and mux configs already work: omitted optional fields are cleared, an omitted `relays` removes all relays, an omitted `tags` empties them, and an omitted `active` resets it to its default `true`. `pattern` stays required.
- **Proposer path key is a validated BLS key.** `GET`/`PUT`/`DELETE /api/admin/vouch/proposers/{public_key}` parse it, reject an invalid one with `400`, and normalize it to lower case, so a proposer is always stored under the form Vouch sends.
- OpenAPI documents the error responses (`400`/`401`/`404`/`409` with the error body) on every operation.

**Effect on Vouch and Commit-Boost**: successful responses do not change, and Vouch instances with more than about 20,700 validators, which today get `413`, receive their config. Two error paths change: a request to `POST /vouch/v2/execution-config/{config}` with a malformed body or an invalid key now gets `400` with a JSON body instead of a plain-text `400`/`422`, and an unknown route anywhere gets a JSON `404` body instead of an empty one. Vouch sends well-formed keys, so neither path is expected in normal operation.

**Database migration**: none. Stored proposer keys that are not lower-case BLS keys keep their current, never-matching state. The implementation includes a read-only query to find them so they can be re-created through the admin API.

## Capabilities

### New Capabilities
- `admin/error-responses`: the error body, status codes and `code` values every endpoint uses, for input errors, missing resources, conflicts, authentication and unknown routes.
- `admin/list-pagination`: the keyset (`limit`/`after`/`next_cursor`) contract of the admin list endpoints and their ordering guarantee.
- `vouch/config-management`: how admin writes create, replace and identify default configs, proposers and proposer patterns: `PUT` as full replacement, duplicate names, and proposer key normalization.

### Modified Capabilities
- `vouch/config-validation`: the "Validated fields" requirement's scenario about omitted fields on update assumed a partial update; under full replacement an omitted field is cleared, so that scenario changes.
- `vouch/execution-config`: gains a requirement on the request size the endpoint accepts.

## Impact

- **Code**:
  - error mapping in `src/errors.rs`, including unique violations mapped to `409`;
  - thin wrapper extractors replacing `Json`, `Query` and `Path` in all handlers;
  - a router fallback for unknown routes and methods;
  - update handlers and request types for default configs and patterns;
  - proposer handlers parsing the path key;
  - keyset pagination in the four list handlers, with `PaginatedResponse` replaced by a `ListResponse { items, next_cursor }`;
  - a 10 MB body limit layer on the public Vouch router.
- **API**:
  - BREAKING for admin clients that send partial `PUT`s, expect `422`, expect `400` on duplicates, or page with `offset`/`total`;
  - public endpoints change only in their error responses;
  - `openapi.json` is regenerated.
- **Dependencies**: may enable Axum's `macros` feature for the extractor wrappers; no new crates.
- **Data**: no migration. A one-off query lists proposer keys stored in a non-canonical form.
