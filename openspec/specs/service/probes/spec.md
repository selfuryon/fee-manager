# service/probes Specification

## Purpose

Lets an orchestrator such as Kubernetes manage the service safely: a liveness endpoint that only says the process is alive, a readiness endpoint that says whether it can serve requests, and a shutdown that does not drop requests in flight.

## Requirements

### Requirement: Liveness
`GET /health` SHALL return `200` with `{"status": "healthy"}` whenever the process is able to answer HTTP, without depending on the database or any other external system, so that an outage elsewhere does not get the process restarted.

#### Scenario: Database down
- **WHEN** the database is unreachable and a client requests `GET /health`
- **THEN** the response is `200` with `{"status": "healthy"}`

### Requirement: Readiness
`GET /ready` SHALL check that the database answers a trivial query within 2 seconds. It SHALL return `200` with `{"status": "ready"}` when the check succeeds, and `503` with the JSON error body and `error.code` `SERVICE_UNAVAILABLE` when the database fails or does not answer in time. The endpoint SHALL NOT require authentication.

#### Scenario: Database available
- **WHEN** the database is reachable and a client requests `GET /ready`
- **THEN** the response is `200` with `{"status": "ready"}`

#### Scenario: Database unreachable
- **WHEN** the database is unreachable and a client requests `GET /ready`
- **THEN** the response is `503` with `error.code` `SERVICE_UNAVAILABLE`, returned within about 2 seconds

### Requirement: Graceful shutdown
On `SIGTERM` or `SIGINT` the process SHALL stop accepting new connections, let requests already in progress complete, and then exit with status `0`.

#### Scenario: Stop signal while idle
- **WHEN** the process receives `SIGTERM` with no request in progress
- **THEN** it exits with status `0` within 1 second

#### Scenario: Stop signal during a request
- **WHEN** the process receives `SIGTERM` while a request is being handled
- **THEN** that request completes with its normal response before the process exits
