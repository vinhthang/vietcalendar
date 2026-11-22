# HTTP REST API and OpenAPI Specification

- Feature ID: 2026-11-21-http-rest-openapi
- Type: enhancement
- Status: In-Testing
- First entered In-Testing: 2026-11-21
- Related specifications: None

## Problem and source intent

Brownfield specification baseline requested by user: "Analyze this project, write AGENTS.md, write specs for existing features using my `spec-driven` skill... document current behavior and catalog discovered quirks/tech debt". Captures the HTTP REST endpoints, query/path parsing, error handling (`AppError`), CORS middleware, and Utoipa OpenAPI / Swagger UI exposure in `src/handlers.rs` and `src/main.rs`.

## Repository grounding

- Legacy `src/handlers.rs` (migrated to `src/plugins/http_{server,calendar,holidays}.rs`): `ErrorResponse`, `AppError`, `LunarQuery`, `HolidayQuery`, `ConvertSolarToLunarQuery`, `ConvertLunarToSolarQuery`, `home`, `get_lunar`, `get_solar_to_lunar`, `get_lunar_to_solar`, `check_vietnam_holiday`.
- [`src/main.rs`](../../../src/main.rs): CLI binary booting Cordis profile routes (`/`, `/lunar`, `/convert/solar-to-lunar/{date}`, `/convert/lunar-to-solar/{date}`, `/vietnam-holiday`, `/swagger-ui`, `/api-docs/openapi.json`).
- Legacy `src/models.rs` (migrated to `src/contracts/models.rs`): `DateMonthYear` response schema (`ToSchema`).
- [`docs/ARCHITECTURE_DECISIONS.md`](../../ARCHITECTURE_DECISIONS.md): ADR-3, ADR-4, ADR-7, ADR-11.

## Non-goals

- Documenting MCP transport endpoints mounted on the same router (`/mcp`, `/mcp/sse`, `/mcp/message`; covered in [mcp-server](../2026-11-21-mcp-server/spec.md)).
- Fixing OpenAPI query alias mismatches, unused `LunarDate` schema registration, or missing `timezone` parameter on `/vietnam-holiday` (`TD-06`) in this documentation-only pass.

## Requirements

### REQ-008: Today's Vietnam Lunar Date Endpoint (`GET /`)

- Source intent: Baseline behavior in `src/handlers.rs::home` and `ADR-4` in [`docs/ARCHITECTURE_DECISIONS.md`](../../ARCHITECTURE_DECISIONS.md).
- Statement: WHEN a client requests `GET /`, THE HTTP API SHALL compute the current calendar date anchored to Vietnam Indochina Time (`(Utc::now() + chrono::Duration::hours(7)).date_naive()`), convert it via `to_lunar(..., 7.0)`, and return HTTP `200 OK` with `DateMonthYear` JSON (`leap: Some(true)` only when `is_leap` is true).
- AC-008-1 — Success: `home()` returns `Json<DateMonthYear>` using UTC+7 offset regardless of host OS timezone, setting `leap` to `Some(true)` only during a lunar leap month and omitting `leap` otherwise.
- AC-008-2 — Boundary/failure: Because `home()` derives the date internally from `Utc::now() + 7h` with no caller input, no validation error path exists on `GET /`.

### REQ-009: Query-Parameter Solar-to-Lunar Conversion (`GET /lunar`)

- Current contract: Changed by [nonblocking-plugin-refactor/REQ-030](../2026-11-21-nonblocking-plugin-refactor/spec.md#req-030-hot-swappable-non-blocking-http-rest-plugins-and-accurate-openapi)
- Source intent: Baseline behavior in `src/handlers.rs::get_lunar` and `LunarQuery` verified by `test_get_lunar_handler` and `test_get_lunar_handler_invalid_date`.
- Statement: WHEN a client requests `GET /lunar` with `dd`, `mm` (or `MM`), `yyyy`, and optional `time_zone` (accepting `timeZone` or `timezone`, defaulting to `7.0`), THE HTTP API SHALL validate the Gregorian date via `NaiveDate::from_ymd_opt` and return `DateMonthYear` JSON or `AppError::BadRequest`.
- AC-009-1 — Success: `GET /lunar?dd=12&mm=9&yyyy=2015` (and uppercase `MM=9`) returns `DateMonthYear { dd: 30, mm: 7, yyyy: 2015, leap: None }`.
- AC-009-2 — Boundary/failure: IF `NaiveDate::from_ymd_opt(q.yyyy, q.mm, q.dd)` returns `None` (e.g., `dd=31, mm=2, yyyy=2024`), THEN `get_lunar` SHALL return `AppError::BadRequest("Invalid solar date: 31/02/2024")` (`HTTP 400`).

### REQ-010: Symmetric ISO-8601 Path Conversion Endpoints (`/convert/.../{date}`)

- Current contract: Changed by [nonblocking-plugin-refactor/REQ-030](../2026-11-21-nonblocking-plugin-refactor/spec.md#req-030-hot-swappable-non-blocking-http-rest-plugins-and-accurate-openapi)
- Source intent: Baseline behavior in `src/handlers.rs::get_solar_to_lunar`, `get_lunar_to_solar` and `ADR-7` in [`docs/ARCHITECTURE_DECISIONS.md`](../../ARCHITECTURE_DECISIONS.md).
- Statement: WHEN clients request `GET /convert/solar-to-lunar/{date}` or `GET /convert/lunar-to-solar/{date}` using `YYYY-MM-DD` path syntax, THE HTTP API SHALL convert solar-to-lunar with optional `timezone` (default `7.0`) or lunar-to-solar with optional `leap` (default `false`) and `timezone` (default `7.0`), returning `DateMonthYear` JSON or `AppError::BadRequest`.
- AC-010-1 — Success: `/convert/solar-to-lunar/2015-09-12` returns `30/7/2015`, `/convert/lunar-to-solar/2015-07-30` returns `12/9/2015`, and `/convert/lunar-to-solar/2004-02-11?leap=true` returns `31/3/2004`.
- AC-010-2 — Boundary/failure: IF `{date}` fails `YYYY-MM-DD` parsing (e.g. `12-09-2015` or `20150730`), has non-numeric parts, falls outside `mm in 1..=12` / `dd in 1..=30`, or fails `to_solar` leap validation, THEN the handler SHALL return `AppError::BadRequest`.

### REQ-011: Vietnam Holiday HTTP Check (`GET /vietnam-holiday`)

- Current contract: Changed by [nonblocking-plugin-refactor/REQ-030](../2026-11-21-nonblocking-plugin-refactor/spec.md#req-030-hot-swappable-non-blocking-http-rest-plugins-and-accurate-openapi)
- Source intent: Baseline behavior in `src/handlers.rs::check_vietnam_holiday` verified by `test_check_vietnam_holiday_handler`.
- Statement: WHEN a client requests `GET /vietnam-holiday` with `dd`, `mm` (or `MM`), and `yyyy`, THE HTTP API SHALL validate the solar date via `NaiveDate::from_ymd_opt` and return a JSON boolean from `is_vietnam_holiday(date, 7.0)`.
- AC-011-1 — Success: `HolidayQuery { dd: 30, mm: 4, yyyy: 2015 }` returns `Json(true)` (`HTTP 200 OK`).
- AC-011-2 — Boundary/failure: IF `NaiveDate::from_ymd_opt(q.yyyy, q.mm, q.dd)` returns `None`, THEN `check_vietnam_holiday` SHALL return `AppError::BadRequest("Invalid date: ...")`.

### REQ-012: Centralized `AppError`, Permissive CORS, and OpenAPI / Swagger UI

- Current contract: Changed by [nonblocking-plugin-refactor/REQ-030](../2026-11-21-nonblocking-plugin-refactor/spec.md#req-030-hot-swappable-non-blocking-http-rest-plugins-and-accurate-openapi)
- Source intent: Baseline behavior in `src/handlers.rs::AppError`, `ErrorResponse`, and `src/main.rs::ApiDoc`, `run_http_server` (`ADR-3`, `ADR-11`).
- Statement: WHEN any handler returns `AppError` or when clients access OpenAPI documentation or cross-origin HTTP endpoints, THE Axum application SHALL map `AppError::BadRequest` to `HTTP 400` and `AppError::NotFound` to `HTTP 404` with JSON `{ "error": "..." }`, apply `CorsLayer` allowing any origin/method/header, and serve OpenAPI 3.0 JSON at `/api-docs/openapi.json` plus Swagger UI at `/swagger-ui`.
- AC-012-1 — Success: `AppError::into_response` emits `StatusCode::BAD_REQUEST` / `StatusCode::NOT_FOUND` with serialized `ErrorResponse`; `ApiDoc` registers all 5 REST handler paths plus schemas (`DateMonthYear`, `LunarDate`, `ErrorResponse`) under `SwaggerUi`.
- AC-012-2 — Boundary/failure: Handlers never return `Result<Json<T>, String>` (preventing `HTTP 200 OK` on application errors), while Axum extractor-level `Query` rejections remain standard Axum plain-text `400` responses as noted in `TD-06`.

## Assumptions and owner-deferred questions

- **Contract quirks (`TD-06`):** `IntoParams` exposes `time_zone` rather than query aliases `timeZone`/`timezone`/`MM`, `models::LunarDate` appears in `ApiDoc` despite HTTP routes returning `DateMonthYear`, `AppError::NotFound` is currently unused, and `GET /vietnam-holiday` hardcodes timezone `7.0`. Documented in [`docs/PROJECT_ANALYSIS.md`](../../PROJECT_ANALYSIS.md) without code modification.

## Approvals and lifecycle history

- 2026-11-21 (`Planning -> In-Progress`): Authorized via `/goal` and settled plan answers ("Advance all 5 baseline specs to In-Testing with real execution evidence") covering `REQ-008..REQ-012` after Gate 1 and Gate 2 passed.
- 2026-11-21 (`In-Progress -> In-Testing`): [Task 1](./tasks.md#task-1) and [Run 1](./verification.md#run-1--2026-11-21) passed; awaiting human confirmation on real-world usage before promoting to `Verified`.
