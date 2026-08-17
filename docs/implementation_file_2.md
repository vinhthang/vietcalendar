# Phase 2 Enhancements for Vietnam Lunar Calendar (Rust)

This plan covers adding observability, OpenTelemetry, API documentation, skipping failing tests, CI/CD setup, and Docker optimization to the newly ported Rust web server.

## Proposed Changes

### 1. Update Core Dependencies to Latest

- Change base HTTP framework directly to Axum 0.8.8 and update other crates with exact numerical latest versions.

#### [MODIFY] `Cargo.toml`
- Update `axum = "0.8.8"`
- Update `tokio = { version = "1.43.0", features = ["full"] }`
- Update `serde = { version = "1.0.217", features = ["derive"] }`
- Update `serde_json = "1.0.138"`
- Update `chrono = { version = "0.4.39", features = ["serde"] }`

---

### 2. Robust Observability, Logging, & OpenTelemetry

- Add tracing crates (`tracing`, `tracing-subscriber`) for structured logging.
- Introduce `opentelemetry` to emit execution traces.
- Add `tower-http` to automatically log incoming HTTP requests and responses.

#### [MODIFY] `Cargo.toml`
- Add logging & tracing dependencies: `tracing = "0.1.41"`, `tracing-subscriber = { version = "0.3.19", features = ["env-filter"] }`, `tower-http = { version = "0.6.2", features = ["trace"] }`.
- Add open telemetry dependencies: `opentelemetry = "0.27.1"`, `opentelemetry_sdk = { version = "0.27.1", features = ["rt-tokio"] }`, `opentelemetry-otlp = { version = "0.27.0", features = ["grpc", "tokio"] }`, `tracing-opentelemetry = "0.28.0"`.

#### [MODIFY] `src/main.rs`
- Initialize `tracing_subscriber::fmt` combined with an OpenTelemetry OTLP tracing layer.
- Wrap the Axum router with `.layer(tower_http::trace::TraceLayer::new_for_http())`.

---

### 3. API Documentation (Swagger / OpenAPI)

#### [MODIFY] `Cargo.toml`
- Add `utoipa = { version = "5.3.1", features = ["chrono"] }`, `utoipa-axum = "0.1.4"`, and `utoipa-swagger-ui = { version = "9.0.0", features = ["axum"] }` (versions that are officially compatible with axum 0.8).

#### [MODIFY] `src/models.rs`
- Add `#[derive(utoipa::ToSchema)]` to `DateMonthYear` and other schema models.

#### [MODIFY] `src/handlers.rs`
- Add `utoipa::path(...)` attribute macros to `home`, `get_lunar`, and `check_vietnam_holiday`.
- Derive `IntoParams` for `LunarQuery` and `HolidayQuery` using `utoipa::IntoParams`.

#### [MODIFY] `src/main.rs`
- Provide an `OpenApi` struct combining all paths and schemas.
- Mount the `SwaggerUi` at `/swagger-ui` in the Axum Router.

---

### 4. Bypass Failing Holiday Test

- The user requested to skip the `test_holiday` test temporarily as it fails currently. It will be checked manually later.

#### [MODIFY] `src/services/tests.rs` (or where the tests reside)
- Locate the test function (e.g. `fn test_holiday()`) and add the `#[ignore]` attribute macro right above it to prevent it from failing the CI pipeline.

---

### 5. CI/CD Automation

#### [NEW] `.github/workflows/rust.yml`
- Add a GitHub Actions workflow with the following jobs:
  - **Fmt**: `cargo fmt --all -- --check`
  - **Clippy**: `cargo clippy -- -D warnings`
  - **Test**: `cargo test` (the ignored test will be gracefully bypassed).
  - **Docker Build Check**: Validate that the container can successfully build.

---

### 6. Optimize Docker for Production

#### [MODIFY] `Dockerfile`
- Ensure a proper multi-stage build block.
- Stage 1: Use `rust:1.94` to compile the app in release mode.
- Stage 2: Copy ONLY the compiled binary into a minimal `debian:bookworm-slim` container.
- Clean up the image to keep size < 40MB.
- Make sure to properly expose the `8080` port and map `TOKIO_WORKER_THREADS`.

## User Review Required

> [!CAUTION]
> The dependencies will all be migrated to **Axum 0.8.8** and the corresponding latest versions for Utoipa 9.0 and Tower-Http 0.6. Are you ready to proceed with these changes?

## Verification Plan

### Automated Tests
- Run `cargo check` and `cargo test` locally to ensure no compilation issues were introduced. Validate that `test_holiday` is marked as ignored.

### Manual Verification
1. Run `cargo run`.
2. Check standard output to verify that OpenTelemetry and tracing layers initialized successfully without panics.
3. Access `http://localhost:8080/swagger-ui` in the browser to verify the Swagger definitions render correctly.
4. Run `docker build -t vietcalendar-rs:latest .` and inspect it using `docker images` to verify it successfully runs and produces a lightweight image.
