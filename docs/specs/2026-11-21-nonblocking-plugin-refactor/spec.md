# Non-Blocking Plugin Refactor and Debt Remediation

- Feature ID: 2026-11-21-nonblocking-plugin-refactor
- Type: refactor
- Status: In-Testing
- First entered In-Testing: 2026-11-21
- Related specifications: Implemented: Changes [astronomical-calendar](../2026-11-21-astronomical-calendar/spec.md), [vietnam-holidays](../2026-11-21-vietnam-holidays/spec.md), [http-rest-openapi](../2026-11-21-http-rest-openapi/spec.md), [mcp-server](../2026-11-21-mcp-server/spec.md), [runtime-observability-ops](../2026-11-21-runtime-observability-ops/spec.md)

## Problem and source intent

User request: "review architecture, I want a clean, none blocking, and performance focus, and a plugin architecture like cordis in deepseek harness used" — followed by `/grill-me` decisions to migrate all VietCalendar domain, HTTP, MCP, and runtime features into decoupled Cordis plugins, eliminate `TD-01`–`TD-08`, adopt `moka` + `ArcSwap` + `criterion` benchmarks, make Vietnam holiday logic strictly data-driven per year (`config/holidays/<year>.yml` for 2024–2026 + weekends-only fallback), and perform a clean break on the legacy internal Rust library API while preserving HTTP and MCP wire compatibility.

## Repository grounding

- Replaces legacy modules `src/calendar.rs`, `src/models.rs`, `src/services.rs`, `src/handlers.rs`, and `src/mcp.rs` with [`src/contracts/`](../../../src/contracts/) and [`src/plugins/`](../../../src/plugins/).
- Cataloged baseline defects remediated here: `TD-01`–`TD-08` in [`docs/PROJECT_ANALYSIS.md`](../../PROJECT_ANALYSIS.md).
- Depends on Cordis runtime contracts specified in [cordis-plugin-kernel](../2026-11-21-cordis-plugin-kernel/spec.md).

## Non-goals

- Preserving legacy internal Rust symbols (`services::is_solar_leap`, `services::is_solar_holiday`, `services::HOLIDAY_CACHE`, or direct `calendar`/`handlers`/`mcp` module paths) — retired per settled clean-break decision.
- Shipping legacy 2011/2015 holiday YAML tables in `config/holidays/` (tests cover 2024–2026 official schedules plus synthetic fixture/plugin-override tests).

## Requirements

### REQ-027: `moka`-Cached `LunarSpan` Engine and Strict Lunar Day Validation

- Changes: [astronomical-calendar/REQ-003](../2026-11-21-astronomical-calendar/spec.md#req-003-bidirectional-solar-and-vietnamese-lunar-conversion)
- Source intent: `TD-03` remediation and non-blocking performance plan ("immutable `Arc<LunarSpan>` + `moka` W-TinyLFU bounded cache").
- Previous contract: `convert_solar_to_lunar` / `convert_lunar_to_solar` recomputed Meeus AA98 trigonometry on every call and allowed day `30` on 29-day *tháng thiếu* months to roll over silently into the next lunar month.
- Statement: WHEN converting between solar and lunar dates via `VietnamCalendarPlugin` (`CalendarService`), THE calendar plugin SHALL resolve immutable `Arc<LunarSpan>` year layouts via a bounded `moka::sync::Cache<(i32, i16), Arc<LunarSpan>>`, compute O(1) bidirectional conversions matching Meeus AA98, validate finite timezones in `[-12.0, 14.0]`, and reject `month` outside `1..=12` or `day` exceeding the lunar month's actual `29` or `30` days (`TD-03`).
- AC-027-1 — Success: `CalendarService::to_lunar` and `CalendarService::to_solar` match raw Jean Meeus AA98 results identically across an exhaustive 15,000-day range (`1995-01-01` through `2035-12-31`) across multiple timezones while validating `lunar_month_days` (`29` or `30`).
- AC-027-2 — Boundary/failure: IF `to_solar` receives `month` outside `1..=12`, `day > lunar_month_days(...)` (e.g., day `30` on a 29-day *tháng thiếu*), invalid `is_leap_month`, or out-of-range/non-finite `time_zone`, THEN it SHALL return `Err(CalendarError::...)` without panicking or rolling over to the next month.

### REQ-028: Clean-Break Contracts and Exact `is_lunar_leap_year` Predicate

- Retires: [astronomical-calendar/REQ-004](../2026-11-21-astronomical-calendar/spec.md#req-004-metonic-cycle-predicate-and-calendar-dto-serialization)
- Source intent: `TD-04` remediation and settled `/grill-me` clean-break decision ("Clean break on internal Rust library API... remove legacy static services facade, is_solar_leap, and is_solar_holiday completely").
- Previous contract: `services::is_solar_leap(year)` evaluated the Metonic `year % 19` formula under a solar-prefixed name alongside legacy `src/models.rs` and `src/services.rs` exports.
- Retirement and migration rationale: Renaming to `CalendarService::is_lunar_leap_year(year, time_zone)` aligns predicate name and astronomical truth (checking whether `LunarSpan` actually contains a leap month, fixing `TD-04`), while `contracts::models::{DateMonthYear, LunarDate}` preserves 100% HTTP/JSON Serde wire compatibility (`mm`/`MM` aliases and skipped `None` `leap`).
- Statement: WHEN evaluating leap years or serializing HTTP/domain DTOs in `src/contracts/`, THE crate SHALL remove `is_solar_leap`, `is_solar_holiday`, and legacy static modules, expose `CalendarService::is_lunar_leap_year`, and preserve `DateMonthYear` and `LunarDate` Serde wire compatibility.
- AC-028-1 — Success: `CalendarService::is_lunar_leap_year` accurately identifies astronomical lunar leap years (e.g., `2004`, `2023`, `2025` true; `2024` false) and `DateMonthYear` / `LunarDate` pass Serde `"MM"`/`"mm"` and optional `leap` serialization tests.
- AC-028-2 — Boundary/failure: Legacy symbols `is_solar_leap` and `is_solar_holiday` and legacy top-level modules (`src/{calendar,services,handlers,mcp,models}.rs`) are completely absent from the crate.

### REQ-029: Data-Driven Per-Year Vietnam Holiday Plugin (`working_weekends` and Fallback)

- Changes: [vietnam-holidays/REQ-005](../2026-11-21-vietnam-holidays/spec.md#req-005-weekend-and-fixed-solar-holiday-classification)
- Source intent: `TD-01`, `TD-02`, `TD-05` remediation and `/grill-me` data-driven holiday decision ("Data-driven per-year schedules only: load each year's exact holiday list exclusively from external config/plugin tables, falling back to weekends-only... ship official schedules for 2024, 2025, and 2026").
- Previous contract: `services.rs` hardcoded pre-2021 rules (`Eve` + 3 Tết days, 1 National Day) cached under a year-only `RwLock<HashMap<i32, HashSet>>` (`TD-02`), lacked swapped working Saturdays (*ngày đi làm bù*), and diverged from `mcp::compute_holiday_list_for_year` (`TD-01`).
- Statement: WHEN checking holiday status or listing annual schedules via `VietnamHolidaysPlugin` (`HolidayService`), THE holiday plugin SHALL load per-year YAML files (`config/holidays/<year>.yml`, shipping official `2024.yml`, `2025.yml`, `2026.yml`), support reversible plugin overrides via `ctx.effect`, honor `working_weekends` (*làm bù*) to return `false` on swapped `Sat`/`Sun` workdays, fall back to weekends-only (`Sat`/`Sun`) for unconfigured years, and cache immutable `Arc<YearHolidaySchedule>` entries in `moka` invalidated atomically on reload/override.
- AC-029-1 — Success: `HolidayService` returns `true` for configured 2024–2026 official holidays/bridge days, returns `false` on swapped working Saturdays (`2024-05-04`, `2025-04-26`, `2026-08-22`), returns full `YearHolidaySchedule` entries via `year_holidays(year)` (shared by HTTP and MCP, eliminating `TD-01`), and supports reversible plugin-injected year overrides.
- AC-029-2 — Boundary/failure: IF `year` has no YAML schedule or registered override (e.g., `2099`), THEN `is_holiday(date, tz)` SHALL return `true` solely for `Saturday`/`Sunday` and `false` for all weekdays, and `year_holidays(year)` SHALL return an empty list without mutating `moka` caches incorrectly across timezones (`TD-02` eliminated).

### REQ-030: Hot-Swappable Non-Blocking HTTP REST Plugins and Accurate OpenAPI

- Changes: [http-rest-openapi/REQ-012](../2026-11-21-http-rest-openapi/spec.md#req-012-centralized-apperror-permissive-cors-and-openapi--swagger-ui)
- Source intent: `TD-06` remediation + Cordis pluggable HTTP routes (`HttpServerPlugin`, `HttpCalendarPlugin`, `HttpHolidaysPlugin`, `OpenApiPlugin`).
- Previous contract: `src/main.rs` mounted static routes at startup, Axum `Query` extraction errors returned plain-text `400`, `GET /vietnam-holiday` lacked a `timezone` parameter, `IntoParams` exposed `time_zone` instead of `timezone`, and `ApiDoc` included unused `models::LunarDate`.
- Statement: WHEN HTTP plugins mount onto `HttpServerPlugin` (`HttpRouterService` backed by `ArcSwap<axum::Router>` and `ArcSwap<OpenApi>`), THE HTTP layer SHALL preserve all 5 REST endpoints (`/`, `/lunar`, `/convert/solar-to-lunar/{date}`, `/convert/lunar-to-solar/{date}`, `/vietnam-holiday`), parse ISO path dates via zero-alloc ASCII parsing, support optional `timezone` on `/vietnam-holiday`, return JSON `ErrorResponse` for `JsonQuery<T>` extraction rejections (`TD-06`), serve live composed OpenAPI JSON without unused `LunarDate` schema, and cleanly remove unmounted plugin routes/paths on disposal.
- AC-030-1 — Success: End-to-end `Router::oneshot` requests verify all 5 REST routes, `timezone`/`timeZone` and `mm`/`MM` aliases, live `/api-docs/openapi.json` (no `LunarDate` schema) and `/swagger-ui/`, permissive CORS headers, and immediate `404` / OpenAPI omission upon live disposal of `HttpCalendarPlugin` or `HttpHolidaysPlugin`.
- AC-030-2 — Boundary/failure: IF query deserialization fails (e.g., missing/non-numeric `dd` on `/lunar` or `/vietnam-holiday`) or a lunar day exceeds `29`/`30` days (`TD-03`), THEN the HTTP endpoint SHALL return `HTTP 400 Bad Request` with JSON `{ "error": "..." }` (`ErrorResponse`).

### REQ-031: Pluggable MCP Core Registry and Unified Tool/Resource Plugin

- Current contract: Changed by [mcp-unknown-tool-invalid-params/REQ-044](../2026-11-22-mcp-unknown-tool-invalid-params/spec.md#req-044-unknown-mcp-tool-returns-invalid-params)
- Changes: [mcp-server/REQ-014](../2026-11-21-mcp-server/spec.md#req-014-five-tool-mcp-catalog-toolslist-and-toolscall)
- Source intent: `TD-01`, `TD-03`, `TD-07` remediation and Cordis MCP modularity (`McpCorePlugin` + `McpToolsPlugin` + `McpStdioPlugin`).
- Previous contract: `src/mcp.rs` hardcoded a `match` table, used `compute_holiday_list_for_year` (missing compensatory/bridge days), skipped `1..=12`/`1..=30` lunar validation, cast `i64 as i32`, and hardcoded `calendar://holidays/2026`.
- Statement: WHEN MCP tools and resources execute through `McpCorePlugin` (`ArcSwap`-backed `McpRegistryService`) and `McpToolsPlugin`, THE MCP engine SHALL expose all 5 existing tools plus `calendar://today`, dynamic current-year `calendar://holidays/<year>`, and `resources/templates/list` (`calendar://holidays/{year}`), use checked `try_from` integer conversions, delegate exclusively to `CalendarService` and `HolidayService` (`TD-01`, `TD-03`), and support reversible tool/resource unregistration upon plugin disposal.
- AC-031-1 — Success: `tools/call` for `get_year_holidays(2026)` and `resources/read` for `calendar://holidays/2026` return the complete YAML-backed 2026 holiday schedule (including compensatory `2026-04-27`, bridge `2026-08-31`, and `working_weekends` `2026-08-22`); `resources/templates/list` returns `calendar://holidays/{year}`; `resources/list` reflects the current UTC+7 year dynamically.
- AC-031-2 — Boundary/failure: IF `convert_lunar_to_solar` receives out-of-range integers, month outside `1..=12`, or day `30` on a 29-day lunar month, THEN `McpToolsPlugin` SHALL return JSON-RPC `-32602` without silent rollover or truncation.

### REQ-032: Non-Blocking Bounded MCP HTTP/SSE Session Plugin

- Changes: [mcp-server/REQ-017](../2026-11-21-mcp-server/spec.md#req-017-remote-httpsse-and-direct-http-json-rpc-transports)
- Source intent: `TD-07` remediation and non-blocking session management ("ArcSwap session map, bounded mpsc::channel + try_send backpressure, and sessionId validation on notifications").
- Previous contract: `McpSessionManager` used `tokio::sync::RwLock` + `mpsc::unbounded_channel`, spawned cleanup tasks in `SessionStream::drop`, and returned `202 Accepted` on notifications without checking `sessionId`.
- Statement: WHEN serving MCP over HTTP/SSE (`GET /mcp/sse`, `POST /mcp/message?sessionId=...`) and direct HTTP POST (`POST /mcp`, `POST /mcp/sse`), THE `McpHttpPlugin` SHALL store active session senders in a lock-free `ArcSwap` map, allocate bounded `mpsc::channel` queues, deliver events using non-blocking `try_send`, remove dropped sessions synchronously inside `SessionStream::drop`, and reject unknown `sessionId` parameters on both requests and notifications.
- AC-032-1 — Success: `GET /mcp/sse` emits `event: endpoint`, `POST /mcp/message` delivers responses (`202 Accepted`), `POST /mcp` and `POST /mcp/sse` return synchronous `200 OK` / `204 No Content`, and dropping the SSE stream cleans up the session without spawning background tasks.
- AC-032-2 — Boundary/failure: IF `POST /mcp/message` receives an unknown/closed `sessionId` (even for notifications like `notifications/initialized`), THEN it SHALL return `HTTP 404 Not Found`; IF a client's bounded SSE queue is full, THEN `try_send` SHALL return `HTTP 503 Service Unavailable` without blocking the request task.

### REQ-033: Profile-Driven CLI, Gated Telemetry, Graceful Shutdown, and Hygiene Cleanup

- Changes: [runtime-observability-ops/REQ-018](../2026-11-21-runtime-observability-ops/spec.md#req-018-dual-mode-cli-subcommands-and-port-resolution)
- Source intent: `TD-07` & `TD-08` remediation + Cordis profile CLI (`serve`, `mcp`, graceful `SIGTERM`/`SIGINT` + `TracerProvider::shutdown()`, zero production `.unwrap()`).
- Previous contract: `src/main.rs` built OTLP exporter unconditionally, unwrapped `TcpListener::bind`/`axum::serve`, lacked signal shutdown or `TracerProvider` flush, retained unused `conf/configuration.json`, and `.gitignore` listed tracked `Cargo.lock`.
- Statement: WHEN running `vietcalendar [serve|mcp]` or `vietcalendar-mcp`, THE CLI SHALL load `--config-dir` (`VIETCALENDAR_CONFIG_DIR`, defaulting to `./config` with embedded fallback) and `--profile` (`serve`, `mcp-stdio`, `http-only`), enable OTLP gRPC export only when `OTEL_EXPORTER_OTLP_ENDPOINT` is configured or `otlp_enabled: true`, execute graceful `SIGTERM`/`SIGINT` shutdown calling `Kernel::shutdown()` and `TracerProvider::shutdown()`, enforce zero `.unwrap()`/`.expect()` across production `src/`, remove `conf/configuration.json`, and remove `Cargo.lock` from `.gitignore`.
- AC-033-1 — Success: Both `vietcalendar` and `vietcalendar-mcp` compile cleanly with `RUSTFLAGS="-D warnings"`, zero production `.unwrap()`/`.expect()` calls exist in `src/` (outside `#[cfg(test)]`), `conf/configuration.json` is absent, `.gitignore` does not list `Cargo.lock`, and `SIGTERM`/`SIGINT` initiates clean LIFO kernel disposal.
- AC-033-2 — Boundary/failure: IF `OTEL_EXPORTER_OTLP_ENDPOINT` is unset, THEN `TelemetryPlugin` initializes structured console `tracing` without attempting OTLP collector connections or printing spurious exporter errors; IF port binding fails, THEN `HttpServerPlugin` returns `KernelError` cleanly instead of panicking.

### REQ-034: Updated Two-Stage Container Image with Declarative Config Directory

- Changes: [runtime-observability-ops/REQ-020](../2026-11-21-runtime-observability-ops/spec.md#req-020-multi-stage-dependency-caching-non-root-container-build)
- Source intent: Container packaging alignment for new `src/` layout, `benches/`, and `config/` YAML schedules.
- Previous contract: `Dockerfile` stubbed legacy root modules (`src/{calendar,handlers,mcp,models,services}.rs`) and copied no runtime `config/` directory.
- Statement: WHEN building the container image via `Dockerfile`, THE two-stage `rust:1.97` -> `debian:bookworm-slim` build SHALL stub matching `src/lib.rs`, `benches/`, `src/main.rs`, and `src/bin/vietcalendar-mcp.rs` targets, copy `config/` into `/etc/vietcalendar/config`, export `ENV VIETCALENDAR_CONFIG_DIR=/etc/vietcalendar/config`, and retain non-root `appuser` (`UID 10001`).
- AC-034-1 — Success: `Dockerfile` matches actual Cargo manifest targets (`src/`, `tests/`, `benches/`), copies `config/` to `/etc/vietcalendar/config`, sets `VIETCALENDAR_CONFIG_DIR=/etc/vietcalendar/config`, and runs as `appuser`.
- AC-034-2 — Boundary/failure: `Dockerfile` retains `touch` mtime invalidation before final `cargo build --release` and preserves `/usr/local/bin/vietcalendar-rs` compatibility symlink.

### REQ-035: `criterion` Microbenchmark Suite for Hot-Path Performance

- Source intent: User request ("performance focus") and settled `/grill-me` decision ("criterion benchmarks").
- Statement: WHEN benchmarking core hot paths via `cargo bench`, THE `benches/calendar_bench.rs` suite SHALL measure `CalendarService::to_lunar` (warm), `CalendarService::to_solar` (warm), cold `LunarSpan` computation, `HolidayService::is_holiday` (warm), zero-alloc `parse_ymd_ascii`, and wait-free `Kernel` `ServiceRegistry::get` dispatch.
- AC-035-1 — Success: `cargo bench --offline -- --warm-up-time 1 --measurement-time 2` compiles and executes all 6 Criterion benchmark groups cleanly with exit code `0`.
- AC-035-2 — Boundary/failure: Benchmarks run deterministically offline without network access or modifying `config/` files on disk.

## Assumptions and owner-deferred questions

- **Multi-requirement parent coverage:** `REQ-029` supersedes the baseline holiday model across `vietnam-holidays/REQ-005..REQ-007`, `REQ-030` covers `http-rest-openapi/REQ-009..REQ-012`, `REQ-031` covers `mcp-server/REQ-014..REQ-015`, and `REQ-033` covers `runtime-observability-ops/REQ-018..REQ-019` under the Cordis plugin refactor; when promoted to `In-Testing`, 1-hop `- Current contract:` links will annotate each affected parent requirement.
- **No owner-deferred questions:** All design branches were resolved during `/grill-me`.

## Approvals and lifecycle history

- 2026-11-21 (`Planning -> In-Progress`): Authorized via `/goal` and `/grill-me` scope decisions covering `REQ-027..REQ-035` after Gate 1 and Gate 2 passed.
- 2026-11-21 (`In-Progress -> In-Testing`): Tasks 1–4 and [Run 1](./verification.md#run-1--2026-11-21) passed across 23 unit tests, 6 integration tests, `vietcalendar-mcp` stdio CLI, zero production `.unwrap()` audit, and 6 Criterion microbenchmarks; awaiting human real-world confirmation before promoting to `Verified`.
