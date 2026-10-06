# Spec Delta

## MODIFIED Requirements

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
- **WHEN** an admin creates a default config sending only `name` and `active`
- **THEN** the response is `201` and `gas_limit`, `min_value` and `relays` are absent from the stored config

