# admin/error-responses Specification

## Purpose

Gives every endpoint one error contract — a JSON body with a machine-readable code and a status chosen by cause — so clients handle failures the same way whichever layer of the service detected them.

## Requirements

### Requirement: Error body
Every response with a `4xx` or `5xx` status, from any endpoint and including unknown routes, SHALL have `Content-Type: application/json` and the body `{"error": {"code": <string>, "message": <string>}}`. `message` SHALL be human-readable and SHALL NOT contain internal details such as SQL or stack traces.

#### Scenario: Malformed JSON
- **WHEN** a client sends `{"name":` as the body of `POST /api/admin/vouch/configs/default`
- **THEN** the response is `400` with a JSON body whose `error.code` is `INVALID_JSON`

#### Scenario: Unknown route
- **WHEN** a client requests `GET /api/admin/does-not-exist` with a valid token
- **THEN** the response is `404` with a JSON body whose `error.code` is `NOT_FOUND`

#### Scenario: Unsupported method
- **WHEN** a client sends `PATCH /api/admin/vouch/proposers` with a valid token
- **THEN** the response is `405` with a JSON body whose `error.code` is `METHOD_NOT_ALLOWED`

#### Scenario: Database failure
- **WHEN** a request fails because of an unexpected database error
- **THEN** the response is `500` with `error.code` `INTERNAL_ERROR` and a message that does not include the database error text

### Requirement: Input errors are 400
Any rejection of client input SHALL use status `400`, with `error.code`:

- `INVALID_JSON` when the body is not valid JSON or is sent without a JSON content type;
- `INVALID_DATA` when the body is valid JSON but a field is missing, has the wrong type or holds an invalid value (including an invalid BLS public key or Ethereum address), or a path parameter is invalid;
- `INVALID_QUERY` when a query parameter cannot be parsed or is out of range.

The message SHALL identify the offending field or parameter where one can be identified.

#### Scenario: Invalid key in a body
- **WHEN** a client posts `["0x12"]` to `POST /vouch/v2/execution-config/main`
- **THEN** the response is `400` with `error.code` `INVALID_DATA`

#### Scenario: Wrong field type
- **WHEN** a client creates a default config with `"active": "yes"`
- **THEN** the response is `400` with `error.code` `INVALID_DATA` and a message naming `active`

#### Scenario: Unparsable query parameter
- **WHEN** a client requests `GET /api/admin/vouch/proposers?limit=abc`
- **THEN** the response is `400` with `error.code` `INVALID_QUERY`

#### Scenario: Invalid path parameter
- **WHEN** a client requests `DELETE /api/admin/tokens/not-a-uuid`
- **THEN** the response is `400` with `error.code` `INVALID_DATA`

### Requirement: Other error statuses
The service SHALL use these statuses and codes for non-input errors:

- `401` `UNAUTHORIZED` when an admin endpoint is called without a valid bearer token;
- `404` `NOT_FOUND` when the addressed resource or route does not exist;
- `409` `CONFLICT` when a create would duplicate an existing resource's name;
- `413` `PAYLOAD_TOO_LARGE` when a request body exceeds the endpoint's size limit;
- `500` `INTERNAL_ERROR` for unexpected failures;
- `503` `SERVICE_UNAVAILABLE` when the service cannot currently serve requests because a dependency is unavailable.

#### Scenario: Missing token
- **WHEN** a client calls `GET /api/admin/vouch/proposers` without an `Authorization` header
- **THEN** the response is `401` with `error.code` `UNAUTHORIZED`

#### Scenario: Body over the limit
- **WHEN** a client sends a 3 MB body to `POST /api/admin/vouch/configs/default`
- **THEN** the response is `413` with a JSON body whose `error.code` is `PAYLOAD_TOO_LARGE`

#### Scenario: Missing resource
- **WHEN** a client requests `GET /api/admin/vouch/configs/default/unknown-example`
- **THEN** the response is `404` with `error.code` `NOT_FOUND`

#### Scenario: Dependency unavailable
- **WHEN** the database is unreachable and a client requests `GET /ready`
- **THEN** the response is `503` with `error.code` `SERVICE_UNAVAILABLE`
