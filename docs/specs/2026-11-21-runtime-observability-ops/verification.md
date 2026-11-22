# Verification: Runtime CLI, Observability, Containerization, and CI/CD

Governing specification: [spec.md](./spec.md)

## Planned checks

### AC-018-1 (REQ-018)

- Method: Binary compilation check (`cargo test --offline`) and source inspection of `Cargo.toml`, `src/main.rs:41-64,106-111,151-168`, and `src/bin/vietcalendar-mcp.rs`.
- Expected outcome: Both `vietcalendar` and `vietcalendar-mcp` compile cleanly; port fallback resolves `--port`/`PORT` -> `PORT` -> `HTTP_PORT` -> `"8080"`; default subcommand invokes `run_http_server`.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-018-2 (REQ-018)

- Method: Source inspection of `src/main.rs:156-160` and `conf/configuration.json`.
- Expected outcome: `Commands::Mcp` prints `"MCP server error: ..."` on error; `conf/configuration.json` is not referenced anywhere in `src/`.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-019-1 (REQ-019)

- Method: Source inspection of `src/main.rs::init_telemetry` (`L66-L101`) and `run_http_server` (`L141-L145`).
- Expected outcome: Initializes `EnvFilter` (`RUST_LOG`), `TraceLayer::new_for_http()`, OTLP gRPC `TracerProvider` on success, and `tracing::info!` startup logs.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-019-2 (REQ-019)

- Method: Source inspection of `src/main.rs:70-100`.
- Expected outcome: When `SpanExporter::builder().with_tonic().build()` returns `Err`, prints `"Failed to build otlp exporter: ..."` and falls back to console `fmt::layer()`.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-020-1 (REQ-020)

- Method: Source inspection of `Dockerfile` (`L1-L45`).
- Expected outcome: Two-stage `rust:1.97` -> `debian:bookworm-slim` build sets `appuser` (`UID 10001`), `PORT=8080`, `TOKIO_WORKER_THREADS=2`, `RUST_LOG=info`, `EXPOSE 8080`, symlink `vietcalendar-rs`, and `CMD ["vietcalendar", "serve"]`.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-020-2 (REQ-020)

- Method: Source inspection of `Dockerfile` (`L17-L19`).
- Expected outcome: `RUN touch src/lib.rs src/main.rs src/bin/vietcalendar-mcp.rs` precedes final `cargo build --release`.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-021-1 (REQ-021)

- Method: Source inspection of `.github/workflows/rust.yml` (`L1-L84`).
- Expected outcome: `test` runs `cargo fmt --all -- --check`, `cargo clippy -- -D warnings`, `cargo test --verbose`; `publish` depends on `test` and builds/pushes GHCR tags.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-021-2 (REQ-021)

- Method: Source inspection of `.github/workflows/rust.yml:45` and `load_test.js`.
- Expected outcome: `publish` runs only on `push` or `workflow_dispatch` (skipping PRs); `load_test.js` configures 100 VUs, `p(95)<10`, `rate<0.01`, and accepts `200`/`400` for `/convert/lunar-to-solar/:date`.
- Execution: [Run 1](#run-1--2026-11-21)

## Execution evidence

### Run 1 — 2026-11-21

- Checked: REQ-018, REQ-019, REQ-020, REQ-021 / AC-018-1..AC-021-2 (commit `e95a3d4`, `rustc 1.92.0` / `cargo 1.92.0`)
- Command: `rtk proxy cargo test --offline` (exit code: `0`) + static inspection of `src/main.rs`, `src/bin/vietcalendar-mcp.rs`, `Cargo.toml`, `Dockerfile`, `.github/workflows/rust.yml`, `load_test.js`, `conf/configuration.json`
- Output excerpt:
  ```text
  Compiling vietcalendar-rs v0.1.0 (/home/opc/github/vietcalendar)
   Finished `test` profile [unoptimized + debuginfo] target(s) in 1m 27s
    Running unittests src/lib.rs ... 23 passed; 0 failed
    Running unittests src/main.rs (target/debug/deps/vietcalendar-9247d83e866b2721)
  running 0 tests ... ok
    Running unittests src/bin/vietcalendar-mcp.rs (target/debug/deps/vietcalendar_mcp-6a44857eb6849e9f)
  running 0 tests ... ok
    Running tests/integration_test.rs ... 6 passed; 0 failed
  ```
- Result: AC-018-1 `PASS`, AC-018-2 `PASS`, AC-019-1 `PASS`, AC-019-2 `PASS`, AC-020-1 `PASS`, AC-020-2 `PASS`, AC-021-1 `PASS`, AC-021-2 `PASS` — binary targets compiled (`0` warnings/errors), all 29 crate tests passed, and CLI/telemetry/Docker/CI/k6 configurations verified by inspection (`cargo fmt`/`clippy` components unavailable locally; Docker build/GHCR push/k6 not executed locally).

## Real-world observations

No real-world confirmation has been recorded.
