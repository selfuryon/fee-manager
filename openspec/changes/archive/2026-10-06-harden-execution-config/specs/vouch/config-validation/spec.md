# Spec Delta

## Purpose

Keeps values that Vouch cannot use out of the configuration: the admin API refuses an invalid gas limit, minimum value, relay URL or proposer pattern at write time, so the public execution config only ever serves values that were valid when stored.

## ADDED Requirements

### Requirement: Validated fields
The admin API SHALL validate the following fields wherever they appear in a create or update request for a default config, a proposer or a proposer pattern, including inside each entry of a `relays` map:

- `gas_limit`: a string holding a positive integer in decimal digits that fits in 64 bits, with no sign, whitespace, decimal point or exponent.
- `min_value`: a string holding a non-negative decimal number (an amount of ETH), with no sign, whitespace or exponent.
- relay URL, which is each key of a `relays` map: an absolute URL with scheme `http` or `https` and a host.
- proposer pattern `pattern`: a regular expression that compiles.

A field that is omitted or `null` SHALL NOT be validated; validation applies only to values that are present.

#### Scenario: Valid values are accepted
- **WHEN** an admin creates a default config with `gas_limit` `"30000000"`, `min_value` `"0.1"` and a relay at `"https://relay.example.invalid/"`
- **THEN** the response is `201` and the config is stored

#### Scenario: Non-numeric gas limit is rejected
- **WHEN** an admin creates a default config with `gas_limit` `"lots"`
- **THEN** the response is `400` and nothing is stored

#### Scenario: Zero gas limit is rejected
- **WHEN** an admin updates a proposer with `gas_limit` `"0"`
- **THEN** the response is `400` and the proposer is unchanged

#### Scenario: Non-numeric minimum value is rejected
- **WHEN** an admin creates a proposer pattern with `min_value` `"cheap"`
- **THEN** the response is `400` and nothing is stored

#### Scenario: Negative minimum value is rejected
- **WHEN** an admin creates a default config with `min_value` `"-1"`
- **THEN** the response is `400`

#### Scenario: Zero minimum value is accepted
- **WHEN** an admin creates a default config with `min_value` `"0"`
- **THEN** the response is `201`

#### Scenario: Relay URL without http or https is rejected
- **WHEN** an admin creates a proposer whose `relays` map has the key `"relay.example.invalid"` or `"ftp://relay.example.invalid/"`
- **THEN** the response is `400` and the proposer is not created

#### Scenario: Invalid value inside a relay is rejected
- **WHEN** an admin creates a default config whose relay at `"https://relay.example.invalid/"` has `gas_limit` `"lots"`
- **THEN** the response is `400` and nothing is stored

#### Scenario: Pattern that does not compile is rejected
- **WHEN** an admin creates a proposer pattern with `pattern` `"([unclosed"`
- **THEN** the response is `400` and nothing is stored

#### Scenario: Omitted fields are not validated
- **WHEN** an admin updates a default config sending only `active`
- **THEN** the response is `200` and the other fields are unchanged

### Requirement: Rejection response
A request that fails validation SHALL be rejected as a whole with status `400` and the error body `{"error": {"code": "INVALID_DATA", "message": ...}}`, where the message names the offending field and, for a relay, the relay URL it belongs to. A rejected request SHALL NOT change any stored data.

#### Scenario: Message names the field
- **WHEN** an admin creates a default config with `min_value` `"cheap"`
- **THEN** the response body has `error.code` `"INVALID_DATA"` and an `error.message` that contains `min_value`

#### Scenario: Message names the relay
- **WHEN** an admin creates a default config whose relay at `"https://relay.example.invalid/"` has `min_value` `"cheap"`
- **THEN** the `error.message` contains both `min_value` and `https://relay.example.invalid/`

#### Scenario: Update with an invalid relay leaves existing relays intact
- **WHEN** an admin updates a default config that has relays, sending a new `relays` map that contains one invalid entry
- **THEN** the response is `400` and the stored relays are the same as before the request

### Requirement: Values stored as sent
A value that passes validation SHALL be stored and later returned exactly as it was sent, without normalization: no reformatting of numbers and no rewriting of URLs.

#### Scenario: Minimum value keeps its formatting
- **WHEN** an admin creates a default config with `min_value` `"0.10"`
- **THEN** reading the config back returns `min_value` `"0.10"`

#### Scenario: Relay URL keeps its trailing slash
- **WHEN** an admin creates a default config with a relay at `"https://relay.example.invalid/"`
- **THEN** the execution config served for it lists that relay under the key `"https://relay.example.invalid/"`
