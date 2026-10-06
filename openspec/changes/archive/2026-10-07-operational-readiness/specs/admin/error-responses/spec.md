# Spec Delta

## MODIFIED Requirements

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
