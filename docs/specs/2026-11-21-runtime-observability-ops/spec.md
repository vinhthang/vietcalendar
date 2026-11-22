# Runtime CLI, Observability, Containerization, and CI/CD

- Feature ID: 2026-11-21-runtime-observability-ops
- Type: enhancement
- Status: In-Testing
- First entered In-Testing: 2026-11-21
- Related specifications: None

## Problem and source intent

Brownfield specification baseline requested by user: "Analyze this project, write AGENTS.md, write specs for existing features using my `spec-driven` skill... document current behavior and catalog discovered quirks/tech debt". Captures CLI subcommand/port resolution, OpenTelemetry/tracing instrumentation, multi-stage Docker container packaging, GitHub Actions CI/CD, and `k6` load testing.

## Repository grounding

- [`src/main.rs`](../../../src/main.rs): `Cli`, `Commands`, `init_telemetry`, `run_http_server`, `main`.
- [`src/bin/vietcalendar-mcp.rs`](../../../src/bin/vietcalendar-mcp.rs): Dedicated `vietcalendar-mcp` binary.
- [`Cargo.toml`](../../../Cargo.toml): Binary targets (`vietcalendar`, `vietcalendar-mcp`) and telemetry/CLI dependencies.
- [`Dockerfile`](../../../Dockerfile): Two-stage `rust:1.97` -> `debian:bookworm-slim` container build, non-root `appuser` (`UID 10001`), symlink `vietcalendar-rs`.
- [`.github/workflows/rust.yml`](../../../.github/workflows/rust.yml): `test` job (`fmt`, `clippy`, `cargo test`) and `publish` job (`ghcr.io` push).
- [`load_test.js`](../../../load_test.js) and legacy `conf/configuration.json` (removed in refactor).

## Non-goals

- Modifying `Dockerfile`, `.github/workflows/rust.yml`, `src/main.rs`, or removing `conf/configuration.json` (`TD-07`, `TD-08`) in this documentation-only pass.
- Running live Docker builds, GHCR pushes, or `k6` load benchmarks against a remote collector.

## Requirements

### REQ-018: Dual-Mode CLI Subcommands and Port Resolution

- Current contract: Changed by [nonblocking-plugin-refactor/REQ-033](../2026-11-21-nonblocking-plugin-refactor/spec.md#req-033-profile-driven-cli-gated-telemetry-graceful-shutdown-and-hygiene-cleanup)
- Source intent: Baseline behavior in `src/main.rs::Cli`, `Commands`, `run_http_server`, and `src/bin/vietcalendar-mcp.rs` (`ADR-8`).
- Statement: WHEN launching the `vietcalendar` binary with `serve`, `mcp`, or no subcommand, THE runtime SHALL dispatch `Commands::Mcp` to `mcp::run_stdio_server()` and `Commands::Serve` or `None` to `run_http_server`, resolving bind port in order: `--port` / `PORT` -> `env::var("PORT")` -> `env::var("HTTP_PORT")` -> `"8080"` on `0.0.0.0`.
- AC-018-1 — Success: `Cli` defines `serve` (`--port` / `PORT`) and `mcp` subcommands, defaults omitted subcommand to HTTP server on `0.0.0.0:8080` (overridden by `PORT` or `HTTP_PORT`), and `Cargo.toml` exposes both `vietcalendar` and `vietcalendar-mcp` binaries.
- AC-018-2 — Boundary/failure: `Commands::Mcp` catches `run_stdio_server` I/O errors and reports `"MCP server error: ..."` on `stderr`, while `conf/configuration.json` remains unread by runtime code (`TD-08`).

### REQ-019: Structured Tracing and OpenTelemetry OTLP Fallback

- Current contract: Changed by [nonblocking-plugin-refactor/REQ-033](../2026-11-21-nonblocking-plugin-refactor/spec.md#req-033-profile-driven-cli-gated-telemetry-graceful-shutdown-and-hygiene-cleanup)
- Source intent: Baseline behavior in `src/main.rs::init_telemetry` and `run_http_server` (`docs/implementation_file_2.md`).
- Statement: WHEN starting the HTTP server, THE runtime SHALL initialize `tracing_subscriber` with `EnvFilter` (`RUST_LOG`, defaulting to `info,vietcalendar_rs=debug,tower_http=debug`), attach `TraceLayer::new_for_http()` to the Axum router, and register an OpenTelemetry OTLP gRPC `TracerProvider` only when `SpanExporter::builder().with_tonic().build()` succeeds.
- AC-019-1 — Success: `init_telemetry()` builds a `TracerProvider` backed by `opentelemetry_sdk::runtime::Tokio` when OTLP exporter creation succeeds and logs startup URLs (`/swagger-ui`, `/mcp/sse`) via `tracing::info!`.
- AC-019-2 — Boundary/failure: IF OTLP exporter construction fails, THEN `init_telemetry()` logs `"Failed to build otlp exporter: ..."` to `stderr` and initializes console `tracing_subscriber::fmt::layer()` without panicking (noting `TD-07` missing graceful shutdown / collector gating).

### REQ-020: Multi-Stage Dependency-Caching Non-Root Container Build

- Current contract: Changed by [nonblocking-plugin-refactor/REQ-034](../2026-11-21-nonblocking-plugin-refactor/spec.md#req-034-updated-two-stage-container-image-with-declarative-config-directory)
- Source intent: Baseline configuration in `Dockerfile` and `ADR-6`, `ADR-10` in [`docs/ARCHITECTURE_DECISIONS.md`](../../ARCHITECTURE_DECISIONS.md).
- Statement: WHEN building the production container image via `Dockerfile`, THE build SHALL pre-compile dependencies using dummy `src/` and `tests/` stubs in `rust:1.97`, build release binaries (`vietcalendar`, `vietcalendar-mcp`), package them on `debian:bookworm-slim` with `ca-certificates` and `tzdata`, create symlink `/usr/local/bin/vietcalendar-rs`, and run `CMD ["vietcalendar", "serve"]` as non-root `appuser` (`UID 10001`).
- AC-020-1 — Success: `Dockerfile` runs release dependency pre-build before `COPY src` / `COPY tests`, sets `ENV PORT=8080`, `TOKIO_WORKER_THREADS=2`, `RUST_LOG=info`, `EXPOSE 8080`, and executes under `USER appuser`.
- AC-020-2 — Boundary/failure: `RUN touch src/lib.rs src/main.rs src/bin/vietcalendar-mcp.rs` updates source mtimes after replacing stubs so Cargo does not serve stale dummy binaries from layer cache.

### REQ-021: Automated CI/CD Pipeline and Load Test Harness

- Source intent: Baseline configuration in `.github/workflows/rust.yml` (`ADR-9`) and `load_test.js`.
- Statement: WHEN code is pushed or pull-requested on `main`, `master`, or `rust-port` (or tagged `v*.*.*` / manually dispatched), THE CI workflow SHALL gate on `cargo fmt --all -- --check`, `cargo clippy -- -D warnings`, and `cargo test --verbose` before building and pushing `ghcr.io` multi-tagged Docker images on `push` / `workflow_dispatch`; `load_test.js` SHALL remain configured to stress all 5 REST endpoints up to 100 VUs (`p95 < 10ms`, failure rate `< 1%`).
- AC-021-1 — Success: `.github/workflows/rust.yml` enforces `fmt`, `clippy -D warnings`, and `cargo test` in `test`, and `publish` (`needs: test`) pushes GHCR tags only when `event_name` is `push` or `workflow_dispatch`.
- AC-021-2 — Boundary/failure: Pull requests execute `test` without `publish` (`if: github.event_name == 'push' || github.event_name == 'workflow_dispatch'`), and `load_test.js` accepts both `200` and `400` on randomized `/convert/lunar-to-solar/{date}` inputs.

## Assumptions and owner-deferred questions

- **Local toolchain divergence:** CI runs `rust-toolchain@stable` with `rustfmt` and `clippy`, and `Dockerfile` compiles on `rust:1.97`, whereas the local inspection host runs Red Hat `rustc 1.92.0` / `cargo 1.92.0` without `rustfmt`/`clippy` installed. Baseline runtime checks verify `cargo test --offline` locally and validate `.github/workflows/rust.yml` and `Dockerfile` statically (`TD-07`, `TD-08`).

## Approvals and lifecycle history

- 2026-11-21 (`Planning -> In-Progress`): Authorized via `/goal` and settled plan answers ("Advance all 5 baseline specs to In-Testing with real execution evidence") covering `REQ-018..REQ-021` after Gate 1 and Gate 2 passed.
- 2026-11-21 (`In-Progress -> In-Testing`): [Task 1](./tasks.md#task-1) and [Run 1](./verification.md#run-1--2026-11-21) passed; awaiting human confirmation on real-world usage before promoting to `Verified`.
