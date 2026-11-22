# Design: Non-Blocking Plugin Refactor and Debt Remediation

Sources: [spec.md](./spec.md), [decisions.md](./decisions.md)

## Components

| Requirement / AC | Path and symbol | Responsibility / change |
| --- | --- | --- |
| REQ-027, REQ-028 / AC-027-1..AC-028-2 | `src/contracts/models.rs`, `src/contracts/calendar.rs`, `src/plugins/calendar_vn/{mod,math,layout}.rs` | Clean DTOs (`DateMonthYear`, `LunarDate`, `ErrorResponse`), `CalendarService`, `moka` `Arc<LunarSpan>` cache, 29/30-day checks, exact `is_lunar_leap_year`. |
| REQ-029 / AC-029-1, AC-029-2 | `src/contracts/holidays.rs`, `src/plugins/holidays_vn/{mod,loader}.rs`, `config/holidays/{2024,2025,2026}.yml` | Data-driven `HolidayService` (`ArcSwap` + `moka`), official 2024–2026 YAML schedules + `working_weekends` (*làm bù*), weekends-only fallback. |
| REQ-030 / AC-030-1, AC-030-2 | `src/contracts/http.rs`, `src/plugins/{http_server,http_calendar,http_holidays,openapi}.rs` | `ArcSwap<Router>` & `ArcSwap<OpenApi>`, `JsonQuery<T>` JSON rejections, zero-alloc `parse_ymd_ascii`, optional `timezone` on `/vietnam-holiday`, accurate OpenAPI schemas. |
| REQ-031, REQ-032 / AC-031-1..AC-032-2 | `src/contracts/mcp.rs`, `src/plugins/{mcp_core,mcp_tools,mcp_stdio,mcp_http}.rs` | `ArcSwap` MCP registry, 5 tools + dynamic resources/templates using `HolidayService`/`CalendarService`, non-blocking bounded SSE `McpSessionManager`. |
| REQ-033, REQ-034, REQ-035 / AC-033-1..AC-035-2 | `src/plugins/telemetry.rs`, `src/main.rs`, `src/bin/vietcalendar-mcp.rs`, `src/lib.rs`, `Cargo.toml`, `.gitignore`, `Dockerfile`, `benches/calendar_bench.rs` | Profile CLI, gated OTLP + shutdown flush, zero prod `.unwrap()`, trimmed `tokio`, legacy cleanup, updated `Dockerfile`, Criterion benchmarks. |

## Interfaces and flow

- **Three-Role Seam (`src/contracts/` -> `src/plugins/`):**
  - `CalendarService`: `to_lunar(NaiveDate, f64) -> Result<LunarDate, CalendarError>`, `to_solar(i32, i32, i32, bool, f64) -> Result<NaiveDate, CalendarError>`, `lunar_month_days(i32, i32, bool, f64) -> Result<u8, CalendarError>`, `is_lunar_leap_year(i32, f64) -> Result<bool, CalendarError>`.
  - `HolidayService`: `is_holiday(NaiveDate) -> bool`, `year_holidays(i32) -> Arc<YearHolidaySchedule>`, `has_schedule(i32) -> bool`, `replace_schedules(HashMap<i32, Arc<YearHolidaySchedule>>)`, `register_year_schedule(i32, YearHolidaySchedule) -> Box<dyn FnOnce() + Send>`.
  - `HttpRouterService`: `mount_routes(&self, router, openapi)` returning a LIFO disposer handle that rebuilds the active `ArcSwap<Router>` and `ArcSwap<OpenApi>` from remaining registered fragments.
  - `McpRegistryService`: `register_tool(...)`, `register_resource(...)`, `register_resource_template(...)` returning LIFO disposer handles; `dispatch(JsonRpcRequest) -> Option<JsonRpcResponse>` reads wait-free `ArcSwap` maps.

## Boundaries and recovery

- **Astronomical bounds (`REQ-027`):** `tz_quarter_hours` validates finite `[-12.0, 14.0]`; `LunarSpan` encodes exact `day_count: u8` (`29` or `30`) per month slot so `day > day_count` returns `Err(CalendarError::InvalidLunarDay)` cleanly (`TD-03`).
- **Holiday fallback & overrides (`REQ-029`):** `YearHolidaySchedule::is_holiday_date(d)` checks `working_weekends.binary_search(&d).is_ok()` first (returning `false` even on Sat/Sun), then `holiday_dates.binary_search(&d).is_ok()`, then `Sat`/`Sun`.
- **HTTP & MCP backpressure (`REQ-030`, `REQ-032`):** `JsonQuery<T>` converts Axum `QueryRejection` into `AppError::BadRequest`; `McpSessionManager` returns `404 Not Found` on missing `sessionId` for notifications and `503 Service Unavailable` when `mpsc::Sender::try_send` hits `TrySendError::Full`.

## Dependencies and unresolved design work

None.
