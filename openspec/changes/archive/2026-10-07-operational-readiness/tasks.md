# Tasks

## 1. Readiness

- [x] 1.1 Add `ApiError::ServiceUnavailable` (`503 SERVICE_UNAVAILABLE`) in `src/errors.rs`. Verify with a unit test of its status and code.
- [x] 1.2 Make `get_ready` take state and run a 2-second-bounded `SELECT 1` on the pool per design.md, declaring the `503` response in its utoipa annotation. Verify with integration tests:
  - the existing `test_ready_endpoint` still passes against the live database;
  - a new test that serves `create_router` with a pool from `PgPoolOptions::connect_lazy` pointed at an unreachable address gets `503` with `error.code` `SERVICE_UNAVAILABLE` within 3 seconds;
  - in that same setup, `/health` returns `200`.

## 2. Graceful shutdown

- [x] 2.1 Wire `with_graceful_shutdown(shutdown_signal())` into `src/main.rs`, handling `SIGTERM` and `SIGINT` and logging when it starts, and close the pool after `serve` returns. Verify by building the image, running it against the local database, and running `time podman stop`: it finishes in under 2 seconds instead of 10, the container exits `0` (`podman inspect --format '{{.State.ExitCode}}'`), and the log shows the shutdown line.
- [x] 2.2 Verify that an in-flight request completes: start the binary locally, send a large execution-config request (30,000 keys), send `SIGTERM` while it is in flight, and confirm the request returns `200` and the process then exits `0`.

## 3. Documentation

- [x] 3.1 Regenerate `openapi.json` with `UPDATE_OPENAPI=1 cargo test --test openapi_test` and verify `/ready` documents `200` and `503`.
- [x] 3.2 Add a "Running in Kubernetes" section to the README: which probe to use for liveness and readiness, suggested `periodSeconds`/`failureThreshold`, a `preStop` sleep of 5 seconds and `terminationGracePeriodSeconds`. Verify that the section matches the endpoints' behaviour.

## 4. Integration checks

- [x] 4.1 Run `cargo fmt --all --check`, `SQLX_OFFLINE=true cargo clippy --locked --all-targets -- -D warnings` and `cargo test --locked --all-targets`, and verify all pass.
- [x] 4.2 Run `cargo test --test openapi_test` without `UPDATE_OPENAPI` and verify it passes against the committed `openapi.json`.
