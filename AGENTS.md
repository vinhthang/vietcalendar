# VietCalendar Repository Guidelines (`AGENTS.md`)

Canonical instructions for contributors and AI coding agents working in `vietcalendar-rs`.

---

## 1. Project Overview & Module Map

**VietCalendar** (`vietcalendar-rs` 0.2.0, Rust `edition = "2021"`) is a non-blocking HTTP REST API and Model Context Protocol (MCP) server built on a native Cordis-style plugin kernel:

| Path | Responsibility |
| --- | --- |
| [`src/kernel/`](src/kernel/mod.rs) | Zero-domain Cordis kernel: `Context`, `Fiber` tree, wait-free `ArcSwap` `ServiceRegistry`, LIFO `EffectScope`, `EventBus`, and YAML `reconcile`. |
| [`src/contracts/`](src/contracts/mod.rs) | Service Definition traits (`CalendarService`, `HolidayService`, `CanChiSolarService`, `AstrologyService`, `TrachNhatService`, `LunarRecurrenceService`, `RetailCalendarService`, `HttpRouterService`, `McpRegistryService`) and DTOs (`DateMonthYear`, `LunarDate`, `BilingualLabel`, `YearHolidaySchedule`, `ErrorResponse`). |
| [`src/plugins/calendar_vn/`](src/plugins/calendar_vn/mod.rs) | `VietnamCalendarPlugin`: Meeus AA98 math + `moka`-cached `Arc<LunarSpan>`, O(1) conversion, strict 29/30-day validation, `is_lunar_leap_year`. |
| [`src/plugins/holidays_vn/`](src/plugins/holidays_vn/mod.rs) | `VietnamHolidaysPlugin`: async YAML loader (`config/holidays/<year>.yml`) + `ArcSwap`/`moka` engine with `working_weekends` (*làm bù*) and weekends-only fallback. |
| [`src/plugins/canchi_solar_vn/`](src/plugins/canchi_solar_vn/mod.rs) | `CanChiSolarPlugin`: shared JDN Can Chi, Ngũ Hổ Độn, Ngũ Thử Độn, 30 Nạp Âm, and `moka`-cached 24 Solar Terms (*Tiết Khí*) AA98 solver. |
| [`src/plugins/astrology_vn/`](src/plugins/astrology_vn/mod.rs) | `AstrologyPlugin`: BaZi Four Pillars (*Tiết Khí* bounds, Hidden Stems, Ten Gods, 5 Elements, Đại Vận) + Vietnamese Tử Vi Nam Phái Core (~50 major stars, 12 palaces). |
| [`src/plugins/trach_nhat_vn/`](src/plugins/trach_nhat_vn/mod.rs) | `TrachNhatPlugin`: Hoàng Đạo hours/days, 12 Trực, 28 Tú, major taboos, Tam Tai/Kim Lâu/Hoang Ốc/Xung Tuổi checks, and scored `<= 366`-day search. |
| [`src/plugins/recurrence_vn/`](src/plugins/recurrence_vn/mod.rs) | `LunarRecurrencePlugin`: stateless `<= 50`-year lunar recurrence with `Day30Policy` / `LeapMonthPolicy` and folded CRLF RFC 5545 `.ics` export. |
| [`src/plugins/retail_vn/`](src/plugins/retail_vn/mod.rs) | `RetailCalendarPlugin`: wait-free `ArcSwap` Vietnam retail calendar from validated `config/retail_events.yml` + monthly Mùng 1/Rằm peaks. |
| [`src/plugins/http_server.rs`](src/plugins/http_server.rs) | `HttpServerPlugin`: wait-free `ArcSwap<Router>` & `ArcSwap<OpenApi>`, `DynamicRouterService`, `AppError`, `JsonQuery<T>`, graceful shutdown. |
| [`src/plugins/http_calendar.rs`](src/plugins/http_calendar.rs) | `HttpCalendarPlugin`: `/`, `/lunar`, `/convert/solar-to-lunar/{date}`, `/convert/lunar-to-solar/{date}`, zero-alloc `parse_ymd_ascii`. |
| [`src/plugins/http_discovery.rs`](src/plugins/http_discovery.rs) | `HttpDiscoveryPlugin`: dynamic `/llms.txt`, `/llms-full.txt`, and `/.well-known/mcp.json` rendered from live OpenAPI & MCP snapshots. |
| [`src/plugins/http_holidays.rs`](src/plugins/http_holidays.rs) | `HttpHolidaysPlugin`: `/vietnam-holiday` with optional `timezone`/`timeZone`. |
| [`src/plugins/http_niches.rs`](src/plugins/http_niches.rs) | `HttpCanChiPlugin`, `HttpAstrologyPlugin`, `HttpTrachNhatPlugin`, `HttpRecurrencePlugin`, `HttpRetailPlugin`: 10 niche REST routes + OpenAPI fragments. |
| [`src/plugins/openapi.rs`](src/plugins/openapi.rs) | `OpenApiPlugin`: dynamic `/api-docs/openapi.json` and `/swagger-ui`. |
| [`src/plugins/mcp_core.rs`](src/plugins/mcp_core.rs) | `McpCorePlugin`: wait-free `ArcSwap` JSON-RPC `2.0` tool/resource/template dispatcher (`2024-11-05`) with live `initialize.instructions`. |
| [`src/plugins/mcp_tools.rs`](src/plugins/mcp_tools.rs) | `McpToolsPlugin`: registers 5 core tools, `calendar://today`, dynamic current-year `calendar://holidays/<year>`, and `calendar://holidays/{year}` template. |
| [`src/plugins/mcp_niches.rs`](src/plugins/mcp_niches.rs) | `McpCanChiPlugin`, `McpAstrologyPlugin`, `McpTrachNhatPlugin`, `McpRecurrencePlugin`, `McpRetailPlugin`: 11 niche MCP tools & 2 templates (`canchi://solar-terms/{year}`, `retail://calendar/{year}`). |
| [`src/plugins/mcp_stdio.rs`](src/plugins/mcp_stdio.rs) | `McpStdioPlugin`: cancellation-aware `stdin`/`stdout` JSON-RPC loop + `StdioExitSignal`. |
| [`src/plugins/mcp_http.rs`](src/plugins/mcp_http.rs) | `McpHttpPlugin`: `ArcSwap` sessions, bounded `mpsc::channel` + `try_send` (`503` backpressure, `404` notification session check). |
| [`src/plugins/config_reloader.rs`](src/plugins/config_reloader.rs) | `ConfigReloaderPlugin`: non-blocking debounced `notify` + `SIGHUP` fail-safe YAML hot-reload. |
| [`src/plugins/telemetry.rs`](src/plugins/telemetry.rs) | `TelemetryPlugin`: `tracing` subscriber, gated OTLP gRPC export, LIFO `TracerProvider::shutdown()`. |
| [`src/profiles/mod.rs`](src/profiles/mod.rs) | `PluginFactoryRegistry` + YAML loader (`config/cordis.yml`, `config/cordis.patch.yml`) booting `serve`, `mcp-stdio`, or `http-only`. |
| [`llms.txt`](llms.txt) | Root `llmstxt.org` agent discovery guide linking docs, live MCP transports, and discovery URLs. |
| [`config/`](config/cordis.yml) | Declarative Cordis profiles, patch overrides, retail calendar (`config/retail_events.yml`), and official 2024–2026 Vietnam holiday YAML tables (`config/holidays/`). |
| [`tests/integration_test.rs`](tests/integration_test.rs) | End-to-end profile, HTTP `oneshot`, OpenAPI, MCP SSE/POST, discovery, live unmount, and hot-reload integration tests. |
| [`benches/calendar_bench.rs`](benches/calendar_bench.rs) | Criterion microbenchmark suite. |

---

## 2. Mandatory First Step: Read the Documentation

Before modifying code, changing APIs, or adding features, read these project references:

1. **[`docs/specs/INDEX.md`](docs/specs/INDEX.md)**: Active five-document feature specifications (`REQ-001`–`REQ-043`, `ADR-001`–`ADR-013`) governing current behavior and delta chains.
2. **[`docs/ARCHITECTURE_DECISIONS.md`](docs/ARCHITECTURE_DECISIONS.md)**: Architecture Decision Records (ADR 1–13) and Cordis plugin architecture diagram.
3. **[`docs/PROJECT_ANALYSIS.md`](docs/PROJECT_ANALYSIS.md)**: Technical audit, architecture baseline, and remediation summary (`TD-01`–`TD-08` resolved).
4. **[`docs/RUST_STYLE_GUIDE.md`](docs/RUST_STYLE_GUIDE.md)**: Idiomatic Rust coding rules and zero-panic runtime invariants.

---

## 3. Core Architectural & Domain Invariants

1. **Zero-Domain Kernel Boundary (`src/kernel/`):**
   - `src/kernel/` must NEVER import `contracts`, `plugins`, `profiles`, `axum`, `chrono`, or `utoipa`. Plugins communicate strictly through [`src/contracts/`](src/contracts/mod.rs) traits via `ctx.provide` / `ctx.inject`.
   - Every side effect (routes, OpenAPI fragments, MCP tools/resources, schedule overlays, background tasks, OTLP provider) must register a LIFO disposer (`ctx.on_dispose` / `ctx.on_dispose_sync` / `ctx.spawn`) so disposal and live reconfiguration leak nothing.
2. **Non-Blocking Hot Path & Zero-Panic Policy:**
   - Never use `std::sync::RwLock` or `tokio::sync::RwLock` in HTTP/MCP request hot paths, and never introduce `.unwrap()`, `.expect()`, or unchecked indexing in production `src/` files.
   - Use wait-free `ArcSwap`, bounded `moka::sync::Cache`, and bounded `mpsc::try_send` channels (`503` on saturation).
3. **Lunar Date Representation & Validation:**
   - **NEVER** use `chrono::NaiveDate` for lunar dates; always use [`LunarDate`](src/contracts/models.rs#L30-L39).
   - Pass `is_leap_month = true` **only** for the leap month itself (*tháng nhuận*); `CalendarService::to_solar` strictly enforces `month in 1..=12` and exact 29/30-day lunar month bounds (`LunarSpan`).
4. **Data-Driven Per-Year Vietnam Holidays (`config/holidays/<year>.yml`):**
   - `HolidayService` loads per-year schedules exclusively from YAML tables or reversible plugin overrides, honoring `working_weekends` (*ngày đi làm bù*, returning `false` even on Sat/Sun). Unconfigured years fall back strictly to weekends (`Sat`/`Sun`).
   - HTTP `/vietnam-holiday` and MCP `check_vietnam_holiday` / `get_year_holidays` / `calendar://holidays/{year}` share `HolidayService`.
5. **HTTP & MCP Error Contracts:**
   - HTTP handlers return `Result<Json<T>, AppError>` and extract queries via `JsonQuery<T>` so validation and query syntax rejections emit `HTTP 400` `{ "error": "..." }`.
   - `GET /` and MCP `get_today_lunar` / `calendar://today` anchor current date to Vietnam UTC+7.

---

## 4. Rust Coding Style Quick Rules

Follow **[`docs/RUST_STYLE_GUIDE.md`](docs/RUST_STYLE_GUIDE.md)** for **all new and modified Rust code** (`src/`, `tests/`, `benches/`):

- **Formatting & linting:** Match default `rustfmt` (`cargo fmt --all -- --check`) and `cargo clippy -- -D warnings` (`RUSTFLAGS="-D warnings"` locally). Group imports `std` -> external crates -> `crate::` / `vietcalendar_rs::`; forbid wildcard imports outside `#[cfg(test)] mod tests`.
- **Zero-panic runtime:** Zero `.unwrap()`, `.expect()`, `panic!()`, or unchecked indexing on production paths (`src/`).
- **Type safety & naming:** Standard `snake_case` / `UpperCamelCase` / `SCREAMING_SNAKE_CASE`; no `#[allow(non_snake_case)]` attributes; checked integer conversions (`try_from`).
- **Rustdoc:** Require `///` doc comments on all `pub` items and `//!` module docstrings; never use JavaDoc comments.

---

## 5. Spec-Driven Workflow & Breaking Change Protocol

1. **Specifications (`docs/specs/`):** Behavior changes, defect fixes, and new plugins affecting settled contracts (`First entered In-Testing` set) must use a linked delta package in `docs/specs/` and keep [`docs/specs/INDEX.md`](docs/specs/INDEX.md) synchronized. Never edit frozen `REQ-NNN` / `AC-NNN-M` bodies in place.
2. **Breaking Changes (`docs/ARCHITECTURE_DECISIONS.md`):** Record new ADRs and keep the architecture diagram and verification matrix updated.
3. **Synchronize `README.md` & OpenAPI (`utoipa`):** Keep endpoint tables, YAML profile docs, `#[utoipa::path(...)]` macros, `IntoParams`/`ToSchema` derives, and plugin OpenAPI fragments aligned.
4. **TDD, Mutation, Architecture & Code Review Gates:** Plan and implement all executable Rust changes using skill `test-driven-development` (Red -> Green -> Refactor, writing a failing test before production edits and never bypassing failures with `#[ignore]`), audit modified logic with `mutation-guard` (`cargo mutants`) and `arch-guard`, and run a `code-review` pass before completion. Add Criterion microbenchmarks for hot-path changes.
5. **User Communication:** Flag breaking changes explicitly with GitHub `> [!WARNING]` alerts.

---

## 6. Build, Test, Lint, Bench & Run Commands

- **Toolchain note:** `Dockerfile` builds on `rust:1.97`; GitHub Actions runs `rust-toolchain@stable` with `rustfmt` and `clippy`; `Cargo.toml` uses `edition = "2021"` with trimmed `tokio` features.

```bash
# Run unit, integration & bench smoke tests with warnings treated as errors
RUSTFLAGS="-D warnings" cargo test --verbose --all-targets

# Format & lint checks (enforced in .github/workflows/rust.yml)
cargo fmt --all -- --check
cargo clippy -- -D warnings

# Run Criterion microbenchmarks
cargo bench --bench calendar_bench

# Run HTTP REST + Remote MCP SSE server (default profile: serve, config dir: ./config)
cargo run --bin vietcalendar -- serve

# Run with custom config directory or profile (e.g. http-only, mcp-stdio)
cargo run --bin vietcalendar -- serve --config-dir ./config --profile http-only --port 8080

# Run MCP stdio server locally
cargo run --bin vietcalendar-mcp

# Build Docker image
docker build -t vietcalendar-rs .

# Validate all five-document specs
python3 /home/opc/.gemini/config/plugins/spec-driven-plugin/skills/spec-driven/scripts/validate_specs.py docs/specs --all
```
