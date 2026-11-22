# Design: HTTP REST API and OpenAPI Specification

Sources: [spec.md](./spec.md), [decisions.md](./decisions.md)

## Components

| Requirement / AC | Path and symbol | Responsibility / change |
| --- | --- | --- |
| REQ-008 / AC-008-1, AC-008-2 | `src/handlers.rs::home`, `src/main.rs::run_http_server` | `GET /` computes `(Utc::now() + 7h).date_naive()` and calls `to_lunar(..., 7.0)`. |
| REQ-009 / AC-009-1, AC-009-2 | `src/handlers.rs::get_lunar`, `LunarQuery` | `GET /lunar` validates `(yyyy, mm, dd)` via `NaiveDate::from_ymd_opt` and converts using `time_zone` (default `7.0`). |
| REQ-010 / AC-010-1, AC-010-2 | `src/handlers.rs::get_solar_to_lunar`, `get_lunar_to_solar`, `ConvertSolarToLunarQuery`, `ConvertLunarToSolarQuery` | Symmetric `YYYY-MM-DD` path endpoints with `?timezone=` and `?leap=` support. |
| REQ-011 / AC-011-1, AC-011-2 | `src/handlers.rs::check_vietnam_holiday`, `HolidayQuery` | `GET /vietnam-holiday` validates `(yyyy, mm, dd)` and calls `is_vietnam_holiday(date, 7.0)`. |
| REQ-012 / AC-012-1, AC-012-2 | `src/handlers.rs::AppError`, `ErrorResponse`; `src/main.rs::ApiDoc`, `SwaggerUi`, `CorsLayer` | Maps domain errors to `HTTP 400`/`404` JSON, serves Swagger UI & OpenAPI JSON, enables permissive CORS. |

## Interfaces and flow

- **Routes (`src/main.rs`):**
  - `GET /` -> `handlers::home` -> `Json<DateMonthYear>`
  - `GET /lunar` -> `Query<LunarQuery>` (`dd`, `mm`/`MM`, `yyyy`, `timeZone`/`timezone`) -> `Result<Json<DateMonthYear>, AppError>`
  - `GET /convert/solar-to-lunar/{date}` -> `Path<String>` (`%Y-%m-%d`) + `Query<ConvertSolarToLunarQuery>` -> `Result<Json<DateMonthYear>, AppError>`
  - `GET /convert/lunar-to-solar/{date}` -> `Path<String>` (`YYYY-MM-DD`) + `Query<ConvertLunarToSolarQuery>` (`leap`, `timezone`/`timeZone`) -> `Result<Json<DateMonthYear>, AppError>`
  - `GET /vietnam-holiday` -> `Query<HolidayQuery>` (`dd`, `mm`/`MM`, `yyyy`) -> `Result<Json<bool>, AppError>`
  - `GET /swagger-ui`, `GET /api-docs/openapi.json` via `SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi())`.
- **Middleware (`src/main.rs`):** `CorsLayer::new().allow_methods(Any).allow_origin(Any).allow_headers(Any)` and `TraceLayer::new_for_http()`.

## Boundaries and recovery

- **`AppError` mapping (`src/handlers.rs`):** `BadRequest(String)` emits `StatusCode::BAD_REQUEST` + `Json(ErrorResponse { error })`; `NotFound(String)` emits `StatusCode::NOT_FOUND` + `Json(ErrorResponse { error })`.
- **Input validation:** `get_lunar_to_solar` splits `date_str` on `'-'`, requires 3 integers (`yyyy`, `mm`, `dd`), checks `1..=12` and `1..=30`, and maps `to_solar(...) == None` to `AppError::BadRequest`.
- **Baseline discrepancies (`TD-06`):** Axum query extractor deserialization errors emit plain-text `400`; `IntoParams` renders Rust field `time_zone` rather than aliases; `models::LunarDate` is unused by HTTP handlers despite appearing in `ApiDoc`.

## Dependencies and unresolved design work

None.
