# vouch/execution-config Specification

## Purpose

Defines the execution config Vouch receives from `POST /vouch/v2/execution-config/{config}`: which entries it contains, in what order, and that a single response reflects one consistent state of the stored configuration.

## Requirements

### Requirement: Response assembly
The endpoint SHALL accept a JSON array of validator public keys and an optional `tags` query parameter holding a comma-separated list of tags, and SHALL return an execution config with `version` `2` built as follows:

- `fee_recipient`, `gas_limit`, `min_value` and `relays` come from the active default config named by `{config}`.
- `proposers` contains one entry for each requested key that has a stored proposer config, followed by one entry for each proposer pattern that carries at least one of the requested tags.
- Requested keys without a stored proposer config SHALL be omitted, so Vouch applies the defaults to them.
- Fields with no value, an empty `relays` map and an empty `proposers` list SHALL be omitted from the response; `reset_relays` SHALL appear only when it is `true`.
- A relay marked `disabled: true` SHALL be included with that flag, for Vouch to skip.

#### Scenario: Default config only
- **WHEN** Vouch requests config `main` with an empty key array and no tags
- **THEN** the response has `version` `2`, the top-level fields and relays of `main`, and no `proposers`

#### Scenario: Unknown or inactive default config
- **WHEN** Vouch requests a config name that does not exist or whose config is not active
- **THEN** the response is `404`

#### Scenario: Unknown keys are omitted
- **WHEN** Vouch requests two keys, one with a stored proposer config and one without
- **THEN** `proposers` contains exactly one entry, for the known key

#### Scenario: Tags select patterns with OR logic
- **WHEN** Vouch requests `?tags=pool-a,pool-b` and patterns exist tagged `pool-a`, `pool-b` and `pool-c`
- **THEN** `proposers` contains the `pool-a` and `pool-b` patterns and not the `pool-c` pattern

### Requirement: Deterministic entry order
Entries in `proposers` SHALL appear in a fixed order that depends only on the stored configuration and the request, so that identical requests against unchanged configuration return identical responses:

1. All proposer entries for requested keys come first, ordered by public key.
2. All pattern entries follow, ordered by the position in `tags` of the first requested tag the pattern carries; patterns with the same position are ordered by pattern name.

Keys within each `relays` map SHALL be serialized in a fixed order.

#### Scenario: Proposer entries ordered by public key
- **WHEN** Vouch requests keys `0xbb…` and `0xaa…` (in that order), both with stored proposer configs
- **THEN** the `0xaa…` entry precedes the `0xbb…` entry

#### Scenario: Patterns ordered by first matching tag
- **WHEN** Vouch requests `?tags=lido,dev` and pattern `p-dev` carries `dev` while pattern `p-lido` carries `lido`
- **THEN** the `p-lido` entry precedes the `p-dev` entry, and both follow every proposer entry

#### Scenario: Ties broken by pattern name
- **WHEN** Vouch requests `?tags=pool-a` and patterns `zeta` and `alpha` both carry `pool-a`
- **THEN** the `alpha` entry precedes the `zeta` entry on every request

#### Scenario: Repeated requests are identical
- **WHEN** Vouch sends the same request twice and the configuration does not change in between
- **THEN** both response bodies are byte-for-byte identical

### Requirement: Consistent snapshot
A single response SHALL reflect one consistent state of the stored configuration: it SHALL NOT combine data from before and after a concurrent admin change.

#### Scenario: Concurrent relay replacement
- **WHEN** an admin replaces the relays of a proposer while Vouch is fetching a config that includes that proposer
- **THEN** the proposer's entry lists either all of the old relays or all of the new relays, never a mix and never none

### Requirement: Bounded cost per request
The work the endpoint does to load relays SHALL NOT grow in the number of database round trips with the number of requested keys or matched patterns.

#### Scenario: Many known keys
- **WHEN** Vouch requests 1,000 keys that all have stored proposer configs with relays
- **THEN** the response contains all 1,000 entries with their relays, loaded with the same number of database queries as a request for one key

### Requirement: Request size
The endpoint SHALL accept request bodies of up to 10 MB, enough for about 100,000 validator public keys, so that a Vouch instance sending all of its keys in one request is served. A larger body SHALL be rejected with `413` and the JSON error body with `error.code` `PAYLOAD_TOO_LARGE`. This limit SHALL apply only to this endpoint; the admin endpoints keep their smaller default limit.

#### Scenario: Thirty thousand keys
- **WHEN** Vouch requests a config with 30,000 public keys (a body of about 3 MB)
- **THEN** the response is `200`

#### Scenario: Body over 10 MB
- **WHEN** a client sends a body larger than 10 MB to the endpoint
- **THEN** the response is `413` with `error.code` `PAYLOAD_TOO_LARGE`
