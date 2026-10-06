# Design

## Context

- `get_ready` and `get_health` in `src/handlers/mod.rs` are stateless and always return `200`.
- `src/main.rs` builds a `PgPool` with `max_connections(5)` and sqlx's default 30-second `acquire_timeout`, then runs `axum::serve(listener, app).await` with no shutdown signal. In the distroless image the binary is PID 1, so with no handler installed `SIGTERM` has no effect, and Kubernetes and Podman fall back to `SIGKILL` after the grace period.
- The error model from `consistent-admin-api` gives every error the JSON body via `ApiError`.

## Goals / Non-Goals

**Goals:**
- Readiness reflects whether a request can actually be served, and answers quickly even when the pool is exhausted or the database hangs.
- `SIGTERM` leads to a prompt, clean exit without dropping in-flight requests.

**Non-Goals:**
- A shutdown delay inside the process. The usual race, where the pod stops accepting before endpoints are updated, is handled with a `preStop` sleep in the deployment (documented in the README), not in code.
- Checking anything besides the database. Vouch and Commit-Boost are clients, not dependencies.
- A hard shutdown deadline. Kubernetes' `terminationGracePeriodSeconds` already bounds it, and requests here take milliseconds.

## Decisions

### Readiness: timed `SELECT 1` on the shared pool
`get_ready` takes `State<Arc<AppState>>` and runs `sqlx::query("SELECT 1").execute(&state.pool)` inside `tokio::time::timeout(Duration::from_secs(2), …)`. Success returns `200` with `HealthResponse { status: "ready" }`. An error or timeout returns `ApiError::ServiceUnavailable`, and the cause is logged at `warn`.

Using the shared pool, rather than a dedicated connection, is deliberate: an exhausted pool also means requests cannot be served, so the pod should drop out. The outer timeout keeps the probe well under typical Kubernetes probe timeouts despite the pool's 30-second acquire timeout.

*Alternative considered*: shortening the pool's `acquire_timeout` globally. That would also change how real requests behave under load, which is out of scope here.

### New error variant
`ApiError::ServiceUnavailable` maps to `503 SERVICE_UNAVAILABLE` with the message "Service is not ready". The `/ready` annotation declares the `503` with `ErrorResponse`. The OpenAPI modifier skips `/health` and `/ready` for its generic responses, so this `503` is declared explicitly.

### Graceful shutdown
In `main.rs`, `axum::serve(listener, app).with_graceful_shutdown(shutdown_signal())`. `shutdown_signal()` awaits whichever comes first: `tokio::signal::ctrl_c()` or a `tokio::signal::unix::signal(SignalKind::terminate())` stream. It logs "Shutdown signal received, draining connections". After `serve` returns, `state.pool.close().await` runs, which needs a clone of the pool kept before `AppState` is moved into the router. Then `main` returns normally, so the exit status is `0`.

## Risks / Trade-offs

- [A brief database blip marks every pod not-ready at once] → Vouch then gets connection errors instead of `500`s for the length of the blip, which is no worse. Probe `failureThreshold` in the deployment smooths this; the README suggests values.
- [`/ready` now runs a query on every probe] → One trivial query every few seconds per pod; negligible.
- [Long-lived keep-alive connections delay shutdown] → Axum's graceful shutdown stops idle keep-alive connections; only active requests are waited for.

## Migration Plan

1. Deploy. In the Kubernetes manifests, point `readinessProbe` at `/ready` and `livenessProbe` at `/health`, and add a `preStop` of `sleep 5` with a grace period above it.
2. Rollback is redeploying the previous image. The probes still work against it, since both endpoints exist and answer `200`.
