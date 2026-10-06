# Proposal

## Why

Kubernetes cannot tell when this service is useless or when it is stopping. `/ready` answers `200` without touching the database, so a pod whose PostgreSQL is unreachable keeps receiving traffic, and Vouch gets `500`s instead of being routed to a healthy replica. The binary also ignores `SIGTERM`: measured with `podman stop`, every shutdown waits out the full 10-second grace period and ends in `SIGKILL`, cutting off any request still in flight. In Kubernetes that means every rollout and every eviction is slow and may drop requests.

## What Changes

- `GET /ready` checks that the database answers, with a short timeout. It returns `200` `{"status": "ready"}` when it does, and `503` with the JSON error body (`SERVICE_UNAVAILABLE`) when it does not.
- `GET /health` stays a liveness probe that does not depend on the database, so a database outage marks pods not-ready instead of restarting them in a loop. Its behaviour is unchanged.
- On `SIGTERM` or `SIGINT` the service stops accepting new connections, finishes the requests already in flight, closes its database pool and exits `0`.
- The README documents which probe to use for what, and a suggested `preStop` delay.

**Effect on Vouch and Commit-Boost**: their endpoints and responses do not change. During a database outage or a rollout they are now routed away from pods that cannot serve them, instead of getting errors or dropped connections.

**Database migration**: none.

## Capabilities

### New Capabilities
- `service/probes`: what the liveness and readiness endpoints report, and how the process behaves when asked to stop.

### Modified Capabilities
- `admin/error-responses`: the "Other error statuses" requirement gains `503` `SERVICE_UNAVAILABLE`.

## Impact

- **Code**: `get_ready` in `src/handlers/mod.rs` takes state and runs a timed `SELECT 1`; a `ServiceUnavailable` variant in `src/errors.rs`; graceful shutdown wired in `src/main.rs`.
- **API**: `/ready` can now return `503`; `openapi.json` is regenerated. No other endpoint changes.
- **Dependencies**: none new. Uses Tokio's `signal` feature, already enabled via `full`.
- **Deployment**: probes should point liveness at `/health` and readiness at `/ready`. A short `preStop` sleep is recommended, so endpoints are removed before the process stops accepting connections.
