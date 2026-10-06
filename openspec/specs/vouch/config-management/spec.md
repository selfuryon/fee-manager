# vouch/config-management Specification

## Purpose

Defines how the admin API creates, replaces and identifies the Vouch configuration resources — default configs, proposers and proposer patterns — so that what a client writes is exactly what is stored and later served.

## Requirements

### Requirement: PUT replaces the whole resource
`PUT` on a default config (`/api/admin/vouch/configs/default/{name}`), a proposer (`/api/admin/vouch/proposers/{public_key}`) or a proposer pattern (`/api/admin/vouch/proposer-patterns/{name}`) SHALL replace the stored resource with the request body:

- an optional field that is omitted or `null` is cleared;
- an omitted `relays` removes all relays; an omitted `tags` stores no tags;
- an omitted `active` (default configs) stores `true`, and an omitted `reset_relays` stores `false`;
- `pattern` is required for a proposer pattern.

For default configs and proposer patterns, `PUT` on a name that does not exist SHALL return `404`. For proposers, `PUT` SHALL create the proposer when it does not exist (`201`) and replace it otherwise (`200`).

#### Scenario: Omitted field is cleared
- **WHEN** a default config has `fee_recipient` and `min_value` set and an admin sends `PUT` with only `{"gas_limit": "30000000"}`
- **THEN** the response is `200` and the stored config has `gas_limit` `"30000000"`, no `fee_recipient`, no `min_value`, no relays and `active` `true`

#### Scenario: Relays omitted
- **WHEN** a proposer pattern has two relays and an admin sends `PUT` with `{"pattern": "^pool-example/.*$"}`
- **THEN** the stored pattern has no relays and no tags

#### Scenario: Pattern required
- **WHEN** an admin sends `PUT` to a proposer pattern without `pattern`
- **THEN** the response is `400` with `error.code` `INVALID_DATA` and the pattern is unchanged

#### Scenario: Replace a missing default config
- **WHEN** an admin sends `PUT` to `/api/admin/vouch/configs/default/missing-example`
- **THEN** the response is `404`

### Requirement: Unique names
Creating a default config, a proposer pattern or a mux config whose name is already taken SHALL fail with `409` and `error.code` `CONFLICT`, and SHALL NOT modify the existing resource. When two creates with the same name race, exactly one SHALL succeed and the other SHALL get `409`.

#### Scenario: Duplicate default config
- **WHEN** an admin creates default config `example-a` twice
- **THEN** the second response is `409` with `error.code` `CONFLICT`

#### Scenario: Concurrent creates
- **WHEN** two requests create proposer pattern `example-b` at the same time
- **THEN** one response is `201` and the other is `409`

### Requirement: Proposer key identity
The `{public_key}` path parameter of the proposer endpoints SHALL be a BLS public key: `0x` followed by 96 hexadecimal digits in either case. An invalid key SHALL be rejected with `400` and `error.code` `INVALID_DATA`. A valid key SHALL be normalized to lower case before it is stored or looked up, so a proposer created under either case is served to Vouch and found by `GET` and `DELETE` under either case.

#### Scenario: Upper-case key is served
- **WHEN** an admin creates a proposer with a path key in upper-case hex and Vouch then requests the execution config for the same key in lower case
- **THEN** the response contains that proposer's entry

#### Scenario: Lookup under the other case
- **WHEN** a proposer was created with a lower-case key and an admin requests `GET` with the same key in upper case
- **THEN** the response is `200` with `public_key` in lower case

#### Scenario: Invalid key
- **WHEN** an admin sends `PUT /api/admin/vouch/proposers/not-a-key`
- **THEN** the response is `400` with `error.code` `INVALID_DATA` and nothing is stored
