# Design: Runtime CLI, Observability, Containerization, and CI/CD

Sources: [spec.md](./spec.md), [decisions.md](./decisions.md)

## Components

| Requirement / AC | Path and symbol | Responsibility / change |
| --- | --- | --- |
| REQ-018 / AC-018-1, AC-018-2 | `src/main.rs::Cli`, `Commands`, `run_http_server`, `main`; `src/bin/vietcalendar-mcp.rs::main`; `Cargo.toml` | Dual-mode `clap` CLI (`serve`/`mcp`), port precedence, and standalone `vietcalendar-mcp` binary target. |
| REQ-019 / AC-019-1, AC-019-2 | `src/main.rs::init_telemetry`, `run_http_server` | `RUST_LOG` `EnvFilter`, `TraceLayer::new_for_http()`, and conditional OpenTelemetry OTLP gRPC layer. |
| REQ-020 / AC-020-1, AC-020-2 | `Dockerfile` | Two-stage `rust:1.97` -> `debian:bookworm-slim` image build, dependency pre-cache, `appuser` (`UID 10001`), `vietcalendar-rs` symlink. |
| REQ-021 / AC-021-1, AC-021-2 | `.github/workflows/rust.yml`, `load_test.js` | GitHub Actions CI (`fmt`, `clippy`, `test`, GHCR `publish`) and `k6` multi-stage ramp-up load test script. |

## Interfaces and flow

- **CLI & port resolution (`src/main.rs`):** `Cli::parse()` matches `Some(Commands::Mcp)` -> `mcp::run_stdio_server().await`, `Some(Commands::Serve { port })` -> `run_http_server(port.or(cli.port))`, `None` -> `run_http_server(cli.port)`. Port resolution follows `port_opt -> env::var("PORT") -> env::var("HTTP_PORT") -> "8080"`.
- **Telemetry pipeline (`src/main.rs::init_telemetry`):** Reads `RUST_LOG` (default `"info,vietcalendar_rs=debug,tower_http=debug"`), attempts `SpanExporter::builder().with_tonic().build()`, attaches `tracing_opentelemetry::layer()` when successful, and always initializes `tracing_subscriber::fmt::layer()`.
- **Container & CI flow:** `Dockerfile` compiles stubbed targets first, copies real `src/` and `tests/`, `touch`es entrypoints before final release build, copies binaries to `debian:bookworm-slim`, and defaults `CMD ["vietcalendar", "serve"]`. `.github/workflows/rust.yml` runs `test` on push/PR/dispatch and `publish` (`needs: test`) on push/dispatch.

## Boundaries and recovery

- **OTLP exporter failure:** Logs `"Failed to build otlp exporter: ..."` to `stderr` and proceeds with console tracing only.
- **Stdio error reporting:** `Commands::Mcp` prints `"MCP server error: ..."` to `stderr` rather than panicking (`main()` does not unwrap `run_stdio_server()`).
- **Known runtime/ops debt (`TD-07`, `TD-08`):** `TcpListener::bind` and `axum::serve` unwrap startup/serve errors; no `tokio::signal` graceful shutdown or `TracerProvider` flush exists; `conf/configuration.json` (`8082`) is unused; local host lacks `rustfmt` and `clippy` components installed in CI.

## Dependencies and unresolved design work

None.
