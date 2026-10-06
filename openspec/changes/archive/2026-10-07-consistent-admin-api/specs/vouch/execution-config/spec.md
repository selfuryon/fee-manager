# Spec Delta

## ADDED Requirements

### Requirement: Request size
The endpoint SHALL accept request bodies of up to 10 MB, enough for about 100,000 validator public keys, so that a Vouch instance sending all of its keys in one request is served. A larger body SHALL be rejected with `413` and the JSON error body with `error.code` `PAYLOAD_TOO_LARGE`. This limit SHALL apply only to this endpoint; the admin endpoints keep their smaller default limit.

#### Scenario: Thirty thousand keys
- **WHEN** Vouch requests a config with 30,000 public keys (a body of about 3 MB)
- **THEN** the response is `200`

#### Scenario: Body over 10 MB
- **WHEN** a client sends a body larger than 10 MB to the endpoint
- **THEN** the response is `413` with `error.code` `PAYLOAD_TOO_LARGE`
