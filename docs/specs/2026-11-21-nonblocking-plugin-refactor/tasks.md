# Tasks: Non-Blocking Plugin Refactor and Debt Remediation

Sources: [spec.md](./spec.md), [design.md](./design.md)

## Task 1

**Update Cargo dependencies, contracts layer, and hygiene files**

- [x] Complete the work and its applicable checks.
- Covers: REQ-028, REQ-033 / AC-028-1, AC-028-2, AC-033-1
- Files: `Cargo.toml`, `.gitignore`, `src/contracts/mod.rs`, `src/contracts/models.rs`, `src/contracts/calendar.rs`, `src/contracts/holidays.rs`, `src/contracts/http.rs`, `src/contracts/mcp.rs`
- Dependencies: None
- Blocked by: None
- Work: Update `Cargo.toml` (`arc-swap`, `moka`, `serde_yaml_ng`, `notify`, `tokio-util`, `http`, `tower`, `criterion`, `http-body-util`, `tempfile`, minimal `tokio` features), remove `Cargo.lock` from `.gitignore`, delete `conf/configuration.json`, and define `src/contracts/` service traits and DTOs with Serde unit tests.
- Completion evidence: Contracts Serde/schedule tests, `.gitignore` cleanup, and `conf/` deletion verified in [Run 1](./verification.md#run-1--2026-11-21).

## Task 2

**Implement `VietnamCalendarPlugin` and `VietnamHolidaysPlugin` (`2024`–`2026` YAML)**

- [x] Complete the work and its applicable checks.
- Covers: REQ-027, REQ-028, REQ-029 / AC-027-1, AC-027-2, AC-028-1, AC-029-1, AC-029-2
- Files: `src/plugins/calendar_vn/mod.rs`, `src/plugins/calendar_vn/math.rs`, `src/plugins/calendar_vn/layout.rs`, `src/plugins/holidays_vn/mod.rs`, `src/plugins/holidays_vn/loader.rs`, `config/holidays/2024.yml`, `config/holidays/2025.yml`, `config/holidays/2026.yml`
- Dependencies: Task 1
- Blocked by: None
- Work: Write failing unit tests for 15,000-day Meeus equivalence, strict 29/30-day lunar validation, `is_lunar_leap_year`, 2024–2026 MOLISA holidays + swapped working Saturdays, and unconfigured-year weekend fallback; implement `VietnamCalendarPlugin` (`moka` `Arc<LunarSpan>`) and data-driven `VietnamHolidaysPlugin`.
- Completion evidence: 15,000-day AA98 parity across 3 timezones, day-30 rejection, and 2024–2026 holiday tests passed in [Run 1](./verification.md#run-1--2026-11-21).

## Task 3

**Implement HTTP (`HttpServerPlugin`, Calendar, Holidays, OpenAPI) and MCP (`McpCore`, Tools, Stdio, HTTP/SSE) plugins**

- [x] Complete the work and its applicable checks.
- Covers: REQ-030, REQ-031, REQ-032 / AC-030-1, AC-030-2, AC-031-1, AC-031-2, AC-032-1, AC-032-2
- Files: `src/plugins/mod.rs`, `src/plugins/telemetry.rs`, `src/plugins/http_server.rs`, `src/plugins/http_calendar.rs`, `src/plugins/http_holidays.rs`, `src/plugins/openapi.rs`, `src/plugins/mcp_core.rs`, `src/plugins/mcp_tools.rs`, `src/plugins/mcp_stdio.rs`, `src/plugins/mcp_http.rs`
- Dependencies: Task 2
- Blocked by: None
- Work: Implement `TelemetryPlugin` (gated OTLP + shutdown flush), `ArcSwap<Router>`/`OpenApi` HTTP plugins with `JsonQuery<T>` and zero-alloc `parse_ymd_ascii`, and `ArcSwap` MCP plugins with `HolidayService`-backed `get_year_holidays`, `try_from` bounds, and bounded non-blocking SSE sessions.
- Completion evidence: Unit tests for telemetry, HTTP router hot-swap, ASCII parser, and bounded SSE sessions passed in [Run 1](./verification.md#run-1--2026-11-21).

## Task 4

**Wire entrypoints, remove legacy `.rs` modules, add integration tests, `criterion` benches, and update `Dockerfile`**

- [x] Complete the work and its applicable checks.
- Covers: REQ-028, REQ-030, REQ-031, REQ-032, REQ-033, REQ-034, REQ-035 / AC-028-2, AC-030-1, AC-030-2, AC-031-1, AC-031-2, AC-032-1, AC-032-2, AC-033-1, AC-033-2, AC-034-1, AC-034-2, AC-035-1, AC-035-2
- Files: `src/lib.rs`, `src/main.rs`, `src/bin/vietcalendar-mcp.rs`, `tests/integration_test.rs`, `benches/calendar_bench.rs`, `Dockerfile`
- Dependencies: Task 3
- Blocked by: None
- Work: Replace `src/{calendar,services,handlers,mcp,models}.rs`, wire `src/lib.rs`, `src/main.rs` (`SIGTERM`/`SIGINT` graceful shutdown), and `vietcalendar-mcp`; write comprehensive HTTP `oneshot`, MCP, hot-reload, and Criterion microbenchmark suites; update `Dockerfile` for new module stubs and `/etc/vietcalendar/config`.
- Completion evidence: All 23 unit tests, 6 integration tests, `vietcalendar-mcp` stdio CLI, zero production `.unwrap()` audit, and 6 Criterion microbenchmarks passed in [Run 1](./verification.md#run-1--2026-11-21).

## Gate 2: traceability and readiness

| Requirement | AC | ADR or rationale | Design | Task | Planned verification | Status / reason |
| --- | --- | --- | --- | --- | --- | --- |
| REQ-027 | AC-027-1 | [ADR-009](./decisions.md#adr-009-immutable-lunarspan-cache-with-moka-and-arcswap-registries) | [Components](./design.md#components) | [Task 2](#task-2) | [AC-027-1](./verification.md#ac-027-1-req-027) | PASS — 15k-day AA98 equivalence mapped |
| REQ-027 | AC-027-2 | [ADR-009](./decisions.md#adr-009-immutable-lunarspan-cache-with-moka-and-arcswap-registries) | [Boundaries](./design.md#boundaries-and-recovery) | [Task 2](#task-2) | [AC-027-2](./verification.md#ac-027-2-req-027) | PASS — day-30 & tz validation mapped |
| REQ-028 | AC-028-1 | [ADR-009](./decisions.md#adr-009-immutable-lunarspan-cache-with-moka-and-arcswap-registries) | [Components](./design.md#components) | [Task 1](#task-1), [Task 2](#task-2) | [AC-028-1](./verification.md#ac-028-1-req-028) | PASS — `is_lunar_leap_year` & Serde mapped |
| REQ-028 | AC-028-2 | [Rationale](./decisions.md#architectural-rationale) | [Components](./design.md#components) | [Task 1](#task-1), [Task 4](#task-4) | [AC-028-2](./verification.md#ac-028-2-req-028) | PASS — legacy API removal check mapped |
| REQ-029 | AC-029-1 | [ADR-010](./decisions.md#adr-010-data-driven-per-year-yaml-holiday-tables-with-working_weekends-overrides) | [Interfaces](./design.md#interfaces-and-flow) | [Task 2](#task-2) | [AC-029-1](./verification.md#ac-029-1-req-029) | PASS — 2024–2026 & `working_weekends` mapped |
| REQ-029 | AC-029-2 | [ADR-010](./decisions.md#adr-010-data-driven-per-year-yaml-holiday-tables-with-working_weekends-overrides) | [Boundaries](./design.md#boundaries-and-recovery) | [Task 2](#task-2) | [AC-029-2](./verification.md#ac-029-2-req-029) | PASS — unconfigured-year weekend fallback mapped |
| REQ-030 | AC-030-1 | [ADR-009](./decisions.md#adr-009-immutable-lunarspan-cache-with-moka-and-arcswap-registries) | [Interfaces](./design.md#interfaces-and-flow) | [Task 3](#task-3), [Task 4](#task-4) | [AC-030-1](./verification.md#ac-030-1-req-030) | PASS — HTTP `oneshot`, OpenAPI & unmount mapped |
| REQ-030 | AC-030-2 | [ADR-009](./decisions.md#adr-009-immutable-lunarspan-cache-with-moka-and-arcswap-registries) | [Boundaries](./design.md#boundaries-and-recovery) | [Task 3](#task-3), [Task 4](#task-4) | [AC-030-2](./verification.md#ac-030-2-req-030) | PASS — `JsonQuery` 400 JSON rejection mapped |
| REQ-031 | AC-031-1 | [ADR-010](./decisions.md#adr-010-data-driven-per-year-yaml-holiday-tables-with-working_weekends-overrides) | [Interfaces](./design.md#interfaces-and-flow) | [Task 3](#task-3), [Task 4](#task-4) | [AC-031-1](./verification.md#ac-031-1-req-031) | PASS — `get_year_holidays` & templates mapped |
| REQ-031 | AC-031-2 | [ADR-009](./decisions.md#adr-009-immutable-lunarspan-cache-with-moka-and-arcswap-registries) | [Boundaries](./design.md#boundaries-and-recovery) | [Task 3](#task-3), [Task 4](#task-4) | [AC-031-2](./verification.md#ac-031-2-req-031) | PASS — MCP checked `try_from` & day-30 mapped |
| REQ-032 | AC-032-1 | [ADR-009](./decisions.md#adr-009-immutable-lunarspan-cache-with-moka-and-arcswap-registries) | [Interfaces](./design.md#interfaces-and-flow) | [Task 3](#task-3), [Task 4](#task-4) | [AC-032-1](./verification.md#ac-032-1-req-032) | PASS — SSE/POST & drop cleanup mapped |
| REQ-032 | AC-032-2 | [ADR-009](./decisions.md#adr-009-immutable-lunarspan-cache-with-moka-and-arcswap-registries) | [Boundaries](./design.md#boundaries-and-recovery) | [Task 3](#task-3), [Task 4](#task-4) | [AC-032-2](./verification.md#ac-032-2-req-032) | PASS — `404` notification & `503` backpressure mapped |
| REQ-033 | AC-033-1 | [Rationale](./decisions.md#architectural-rationale) | [Components](./design.md#components) | [Task 1](#task-1), [Task 4](#task-4) | [AC-033-1](./verification.md#ac-033-1-req-033) | PASS — zero `.unwrap()`, warnings, cleanup mapped |
| REQ-033 | AC-033-2 | [Rationale](./decisions.md#architectural-rationale) | [Boundaries](./design.md#boundaries-and-recovery) | [Task 3](#task-3), [Task 4](#task-4) | [AC-033-2](./verification.md#ac-033-2-req-033) | PASS — gated OTLP & clean bind failure mapped |
| REQ-034 | AC-034-1 | [ADR-010](./decisions.md#adr-010-data-driven-per-year-yaml-holiday-tables-with-working_weekends-overrides) | [Components](./design.md#components) | [Task 4](#task-4) | [AC-034-1](./verification.md#ac-034-1-req-034) | PASS — `Dockerfile` config copy & stubs mapped |
| REQ-034 | AC-034-2 | [Rationale](./decisions.md#architectural-rationale) | [Components](./design.md#components) | [Task 4](#task-4) | [AC-034-2](./verification.md#ac-034-2-req-034) | PASS — `Dockerfile` cache `touch` & symlink mapped |
| REQ-035 | AC-035-1 | [ADR-009](./decisions.md#adr-009-immutable-lunarspan-cache-with-moka-and-arcswap-registries) | [Components](./design.md#components) | [Task 4](#task-4) | [AC-035-1](./verification.md#ac-035-1-req-035) | PASS — `criterion` 6-group bench run mapped |
| REQ-035 | AC-035-2 | [ADR-009](./decisions.md#adr-009-immutable-lunarspan-cache-with-moka-and-arcswap-registries) | [Boundaries](./design.md#boundaries-and-recovery) | [Task 4](#task-4) | [AC-035-2](./verification.md#ac-035-2-req-035) | PASS — deterministic offline bench execution mapped |

- Backward scope check: PASS — Tasks 1–4 map strictly to `REQ-027..REQ-035` with serialized sequential dependencies and non-overlapping `- Files:` sets.
- Reference and catalog consistency: PASS — `REQ-027..REQ-035`, `ADR-009, ADR-010`, and `docs/specs/INDEX.md` are consistent.
