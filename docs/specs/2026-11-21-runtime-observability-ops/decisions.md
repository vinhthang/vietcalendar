# Decisions: Runtime CLI, Observability, Containerization, and CI/CD

Governing specification: [spec.md](./spec.md)

## Architectural rationale

- Governs: REQ-018, REQ-019, REQ-020, REQ-021

Runtime operations tie together HTTP REST ([http-rest-openapi](../2026-11-21-http-rest-openapi/spec.md#req-012-centralized-apperror-permissive-cors-and-openapi--swagger-ui)) and MCP transports ([mcp-server](../2026-11-21-mcp-server/spec.md#req-016-local-stdio-json-rpc-transport)) inside a single deployable Rust binary alongside container and CI pipelines. `REQ-019` wires `tracing` + optional OTLP gRPC export so startup proceeds with structured stdout logs when no collector is present, and `REQ-021` pairs GitHub Actions (`fmt`, `clippy`, `test`, GHCR publish) with `load_test.js` SLA thresholds. See `ADR-006` for the packaging and binary strategy.

## ADR-006: Single Dual-Mode Binary with Two-Stage Non-Root Container Packaging

- Governs: REQ-018, REQ-019, REQ-020, REQ-021
- State: Accepted
- Context and constraints: VietCalendar must serve HTTP REST + remote MCP SSE in containerized cloud environments (Cloud Run, Fly.io, VPS) while also running cleanly as a zero-HTTP local `stdio` MCP binary (`vietcalendar mcp` or `vietcalendar-mcp`) with fast incremental Docker builds and minimal runtime surface.
- Viable options: Option A — Compile dual targets (`vietcalendar` with `clap` `serve`/`mcp` subcommands and `vietcalendar-mcp`) from one workspace crate using a two-stage `rust:1.97` -> `debian:bookworm-slim` non-root `Dockerfile`; vs. Option B — Split HTTP and MCP into separate Cargo crates/images or use a single-stage `rust` runtime container.
- Decision: Option A — Single-crate dual-mode binaries packaged in a two-stage dependency-caching `Dockerfile` under `appuser` (`UID 10001`).
- Costs and risks: Keeps shared calendar/holiday logic unified, shrinks runtime images by stripping the Rust toolchain, and caches dependencies across source edits. Known caveats (`TD-07`, `TD-08`): `TcpListener::bind`/`axum::serve` use `.unwrap()` with no graceful shutdown hook, OTLP exporter builds unconditionally, `Cargo.toml` pulls `tokio` `full`, and `conf/configuration.json` remains unused.
- Rejected alternatives: Option B inflates runtime container size (>1 GB with compiler toolchain), loses Cargo layer caching, or duplicates domain logic across independent repositories.
- Affected components: `src/main.rs`, `src/bin/vietcalendar-mcp.rs`, `Cargo.toml`, `Dockerfile`, `.github/workflows/rust.yml`.

## Gate 1: normative review

| Check | Status | Evidence / required correction |
| --- | --- | --- |
| Codebase grounding | PASS | Grounded in `src/main.rs`, `src/bin/vietcalendar-mcp.rs`, `Cargo.toml`, `Dockerfile`, `.github/workflows/rust.yml`, `load_test.js`, and `conf/configuration.json`. |
| Attributable intent and explicit scope | PASS | `REQ-018`..`REQ-021` capture CLI, tracing/OTLP, Docker, CI/CD, and `k6` load harness; live Docker/GHCR/code edits excluded in `Non-goals`. |
| Observable requirements and meaningful acceptance criteria | PASS | `AC-018-1`..`AC-021-2` verify CLI/port resolution precedence, OTLP fallback, Docker cache-invalidation/non-root directives, and CI job gates. |
| Assumptions and owner-deferred questions are explicit | PASS | Host `rustc 1.92.0` vs CI `stable`/Docker `1.97` difference documented explicitly; zero `[PROVISIONAL]` or `TODO` items. |
| Architectural rationale respects requirements | PASS | `ADR-006` and `## Architectural rationale` govern `REQ-018`..`REQ-021` cleanly. |
| IDs, references, and revision route are consistent | PASS | `REQ-018..REQ-021` and `ADR-006` match `docs/specs/INDEX.md`; cross-spec links valid. |
