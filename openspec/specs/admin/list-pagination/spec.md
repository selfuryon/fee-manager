# admin/list-pagination Specification

## Purpose

Defines how the admin list endpoints page through results: a keyset contract (`limit`, an opaque `after` cursor and `next_cursor`) whose walk visits every item exactly once, even while items are being added.

## Requirements

### Requirement: Cursor parameters
The list endpoints for proposers, default configs, proposer patterns and mux configs SHALL accept two optional query parameters:

- `limit`: the maximum number of items to return, between `1` and `1000` inclusive, default `100`;
- `after`: an opaque cursor; when present, only items that come after it in the endpoint's order are returned.

The response SHALL be `{"items": [...], "next_cursor": <string>}`. `next_cursor` SHALL be present when the page holds `limit` items, and its value, passed as `after`, SHALL return the following page. It SHALL be omitted when the page holds fewer than `limit` items. A `limit` outside the range SHALL be rejected with `400` and `error.code` `INVALID_QUERY`. Filters combine with the cursor: a walk with filters visits exactly the matching items.

#### Scenario: First page
- **WHEN** 3 default configs exist and a client lists them with `limit=2`
- **THEN** `items` holds 2 configs and `next_cursor` is present

#### Scenario: Following the cursor
- **WHEN** the client repeats that request with `after` set to the returned `next_cursor`
- **THEN** `items` holds the remaining config and `next_cursor` is absent

#### Scenario: Defaults
- **WHEN** a client lists proposer patterns without `limit` or `after`
- **THEN** at most 100 items are returned, starting from the first in order

#### Scenario: Limit out of range
- **WHEN** a client lists proposers with `limit=0` or `limit=1001`
- **THEN** the response is `400` with `error.code` `INVALID_QUERY`

#### Scenario: Maximum limit
- **WHEN** a client lists mux configs with `limit=1000`
- **THEN** the response is `200`

### Requirement: Key order
Each list endpoint SHALL return items ordered by the resource's identifier: proposers by public key, and default configs, proposer patterns and mux configs by name. A walk that follows `next_cursor` from the first page to the last SHALL return every item that existed for the whole walk exactly once, regardless of items created or deleted elsewhere in the order during the walk.

#### Scenario: Insert during a walk
- **WHEN** a client walks the proposer list with `limit=1` and, after the second page, an admin creates a proposer whose key sorts before every key already returned
- **THEN** the rest of the walk returns every remaining pre-existing proposer exactly once and none twice
