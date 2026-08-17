# Architecture Decision Record (ADR): VietCalendar Rust Architecture & Refactoring

**Status:** Accepted & Implemented  
**Date:** 2026-08-17  
**Context:** Porting, stabilizing, and hardening the Vietnam Lunar Calendar API from Java 11 / Vert.x to Rust with Axum & Tokio.

---

## 1. Context and Problem Statement

The Vietnam Lunar Calendar application was migrated from a Java Vert.x codebase to a modern Rust web application (`axum`, `tokio`, `utoipa`, `tracing`). During code review and testing, several critical domain modeling vulnerabilities, algorithmic flaws, and API inconsistencies were discovered:

1. **Gregorian / Lunar Date Incompatibility:** Attempting to store Lunar dates inside `chrono::NaiveDate` caused panics when lunar months contained 30 days in February (`NaiveDate::from_ymd_opt(y, 2, 30)` returns `None`).
2. **Leap Month Algorithmic Flaw:** In `to_solar`, `leap = 1` was being passed for all 12 months in any leap year, causing non-leap months in leap years to return `(0, 0, 0)` and crash the server.
3. **HTTP 200 OK Returned on Errors:** Axum handlers returning `Result<Json<T>, String>` serialized the `Err(String)` with HTTP status `200 OK` rather than `400 Bad Request`.
4. **Incomplete Holiday Logic:** The holiday calculation lacked compensatory leave (nghỉ bù) when public holidays coincided with weekends, and did not properly account for Lunar New Year's Eve across lunar year boundaries.
5. **Timezone Drift in `home` Endpoint:** Defaulting to server `Local::now()` resulted in UTC evaluation in Docker containers rather than Vietnam Indochina Time (UTC+7).
6. **Docker Cache Invalidation:** Source and dependencies were built in a single step without caching pre-compiled crates.

---

## 2. Decision Drivers

- **Reliability & Zero-Panic Guarantee:** Astronomical calculations and web requests must never crash the service on valid domain inputs.
- **RESTful API Standards:** Strict conformance to HTTP status codes, structured JSON error responses, and OpenAPI 3.0 schema definitions.
- **Cultural and Legal Accuracy:** Full fidelity with Vietnamese Labor Code regarding public holidays, Lunar New Year eve, and compensatory leave.
- **Maintainability & Idiomatic Rust:** Strong typing, Serde case aliases, separation of astronomical math from HTTP transport.
- **CI/CD & Container Performance:** Fast Docker layer caching and container security compliance (non-root execution).

---

## 3. Key Architectural Decisions

### ADR-1: Dedicated Domain Model for Lunar Dates
* **Decision:** Decouple Lunar dates from Gregorian calendar structures. Introduce [`LunarDate`](file:///c:/Users/thang/github/vietcalendar/src/models.rs#L14-L21) (`{ day: i32, month: i32, year: i32, is_leap: bool }`) in [`src/models.rs`](file:///c:/Users/thang/github/vietcalendar/src/models.rs).
* **Rationale:** `chrono::NaiveDate` enforces Gregorian constraints (e.g. max 28/29 days in February, 30 in April). Lunar months are synodic and can have 30 days in any lunar month.

### ADR-2: Independent `is_leap_month` Parameter in `to_solar`
* **Decision:** Change `to_solar` signature to accept `(day, month, year, is_leap_month, time_zone)` in [`src/services.rs`](file:///c:/Users/thang/github/vietcalendar/src/services.rs#L27-L35).
* **Rationale:** In the Vietnamese Luni-Solar calendar, a leap year (năm nhuận) contains one specific leap month (tháng nhuận). Only dates within that specific leap month should be converted with `leap = 1`.

### ADR-3: Centralized Error Mapping via `AppError`
* **Decision:** Implement custom error enum `AppError` (`BadRequest`, `NotFound`) implementing Axum's `IntoResponse` trait to emit `StatusCode::BAD_REQUEST` alongside [`ErrorResponse`](file:///c:/Users/thang/github/vietcalendar/src/handlers.rs#L11-L14) (`{ "error": "..." }`).
* **Rationale:** Enforces correct HTTP 4xx status codes and consistent JSON structure across all API endpoints.

### ADR-4: Fixed Vietnam Timezone Anchor (UTC+7)
* **Decision:** Explicitly calculate current date using `(Utc::now() + Duration::hours(7)).date_naive()` in [`src/handlers.rs`](file:///c:/Users/thang/github/vietcalendar/src/handlers.rs#L44-L53).
* **Rationale:** Eliminates dependency on host machine timezone settings when deployed to cloud containers running in UTC.

### ADR-5: Compensatory Leave & Lunar New Year Eve Calculation
* **Decision:** 
  1. Lunar New Year Eve (Tất niên) is calculated dynamically as `to_solar(1, 1, ly) - 1 day`, correctly handling both 29-day (tháng thiếu) and 30-day (tháng đủ) 12th lunar months.
  2. Public holidays that fall on a Saturday or Sunday dynamically generate compensatory holidays on the subsequent available business days.
* **Rationale:** Satisfies real-world Vietnam legal requirements and un-ignores the full test suite.

### ADR-6: Two-Stage Dockerfile with Dependency Pre-Building
* **Decision:** Pre-compile dependencies using a skeleton `src/lib.rs` and `src/main.rs` before copying source code, and run container as non-root user `appuser` (UID 10001).
* **Rationale:** Decreases Docker build times on code modifications from minutes to seconds and improves container security.

### ADR-7: Explicit Bidirectional Conversion Endpoints (`/convert/...`)
* **Decision:** Replaced the legacy 8-digit path parameter route (`/lunar/{ddMMyyyy}`) with standardized, symmetric ISO-8601 conversion routes:
  1. `GET /convert/solar-to-lunar/{date}`: Accepts ISO date string `YYYY-MM-DD` (e.g. `2015-09-12`).
  2. `GET /convert/lunar-to-solar/{date}`: Accepts `YYYY-MM-DD` (where `YYYY`=lunar year, `MM`=lunar month, `DD`=lunar day) with optional query parameter `?leap=true/false` to handle leap months (tháng nhuận).
* **Rationale:** 
  - Standardizes RESTful path formats to international standard (ISO-8601), eliminating `DD-MM` vs `MM-DD` ambiguity.
  - Provides full bidirectional conversion symmetry with explicit namespace clarity.
  - Validates leap month constraints strictly.

### ADR-8: Model Context Protocol (MCP) Server Support [Alpha]
* **Decision:** Implemented JSON-RPC 2.0 stdio Model Context Protocol (MCP) server capabilities in [`src/mcp.rs`](file:///c:/Users/thang/github/vietcalendar/src/mcp.rs) and created a dual-mode CLI with `clap`:
  - `vietcalendar serve`: Starts Axum HTTP REST web server.
  - `vietcalendar mcp` / `vietcalendar-mcp`: Starts Alpha MCP stdio server exposing 5 tools (`get_today_lunar`, `convert_solar_to_lunar`, `convert_lunar_to_solar`, `check_vietnam_holiday`, `get_year_holidays`) and 2 resources (`calendar://today`, `calendar://holidays/{year}`).
* **Rationale:** Enables direct, native integration with modern AI developer tools (Antigravity, Claude Desktop, Cursor, VS Code) while preserving 100% of the production HTTP web server functionality.


---

## 4. Architecture Diagram

```mermaid
graph TD
    Client[HTTP Client / Frontend] -->|HTTP GET| Router[Axum Router]
    
    subgraph "API & Transport Layer (src/handlers.rs, src/main.rs)"
        Router -->|/swagger-ui| Swagger[OpenAPI / Swagger UI]
        Router -->|/| HomeHandler[home()]
        Router -->|/lunar| LunarQueryHandler[get_lunar()]
        Router -->|/convert/solar-to-lunar/:date| SolarToLunarHandler[get_solar_to_lunar()]
        Router -->|/convert/lunar-to-solar/:date| LunarToSolarHandler[get_lunar_to_solar()]
        Router -->|/vietnam-holiday| HolidayHandler[check_vietnam_holiday()]
        
        LunarQueryHandler -.->|Error| AppError[AppError -> StatusCode 400 + JSON]
        SolarToLunarHandler -.->|Error| AppError
        LunarToSolarHandler -.->|Error| AppError
        HolidayHandler -.->|Error| AppError
    end
    
    subgraph "Domain & Services Layer (src/services.rs, src/models.rs)"
        HomeHandler --> ServiceToLunar[to_lunar]
        LunarQueryHandler --> ServiceToLunar
        SolarToLunarHandler --> ServiceToLunar
        LunarToSolarHandler --> ServiceToSolar[to_solar]
        HolidayHandler --> ServiceHoliday[is_vietnam_holiday]
        
        ServiceToLunar --> DomainLunar[models::LunarDate]
        ServiceToSolar --> DomainSolar[chrono::NaiveDate]
        ServiceHoliday --> HolidayEngine[Vietnam Holiday & Compensatory Engine]
    end
    
    subgraph "Astronomical Algorithms (src/calendar.rs)"
        ServiceToLunar --> AstroMath[Jean Meeus AA98 Algorithms]
        ServiceToSolar --> AstroMath
        ServiceHoliday --> AstroMath
    end
```

---

## 5. Implementation Summary & Verification Walkthrough

### Code Changes Across Modules

| File | Change Description |
| :--- | :--- |
| [`src/models.rs`](file:///c:/Users/thang/github/vietcalendar/src/models.rs) | Added `LunarDate`, updated `DateMonthYear` with `leap: Option<bool>` and Serde casing aliases, with unit tests. |
| [`src/services.rs`](file:///c:/Users/thang/github/vietcalendar/src/services.rs) | Implemented safe `to_lunar`, fixed `to_solar` leap month bug, added weekend compensatory holiday logic, with unit tests. |
| [`src/handlers.rs`](file:///c:/Users/thang/github/vietcalendar/src/handlers.rs) | Added `AppError`, `ErrorResponse`, strongly typed queries, `/convert/solar-to-lunar/{date}` and `/convert/lunar-to-solar/{date}` handlers, with unit tests. |
| [`src/main.rs`](file:///c:/Users/thang/github/vietcalendar/src/main.rs) | Registered conversion routes in Axum router and OpenAPI 3.0 specs. |
| [`src/calendar.rs`](file:///c:/Users/thang/github/vietcalendar/src/calendar.rs) | Astronomical Julian and lunar math, non-leap year validation in `convert_lunar_to_solar`, with unit tests. |
| [`tests/integration_test.rs`](file:///c:/Users/thang/github/vietcalendar/tests/integration_test.rs) | Updated assertions to use `LunarDate`, un-ignored `test_holiday`, added roundtrip conversion tests. |
| [`Dockerfile`](file:///c:/Users/thang/github/vietcalendar/Dockerfile) | Optimized with multi-stage cargo dependency caching and added non-root unprivileged `appuser`. |

### Verification Matrix

| Area | Verification Test | Expected Output | Status |
| :--- | :--- | :--- | :---: |
| **Solar to Lunar (ISO Path)** | `test_get_solar_to_lunar_endpoint` | Converts `YYYY-MM-DD` to `LunarDate` output | Passed |
| **Lunar to Solar (Path & Leap)** | `test_get_lunar_to_solar_endpoint` | Converts lunar date to solar date with `?leap=true/false` support | Passed |
| **Solar to Lunar (Query Params)** | `test_get_lunar_handler` | Parses query parameters and returns `DateMonthYear` | Passed |
| **Julian Day Calculations** | `test_jd_roundtrip` | Accurate across Gregorian reform (1582) and modern centuries | Passed |
| **Roundtrip Math** | `test_roundtrip_conversion` | `to_solar(to_lunar(d)) == d` | Passed |
| **Compensatory Holidays** | `test_holiday`, `test_vietnam_holidays_with_compensatory` | 2011-04-30 (Sat) & 2011-05-01 (Sun) shift to 2011-05-02 (Mon) & 2011-05-03 (Tue) | Passed |
| **Error Handling** | `test_app_error_into_response`, invalid input tests | Returns HTTP 400 with `{ "error": "..." }` | Passed |
| **Domain Serde** | `test_date_month_year_serde`, `test_lunar_date_serde` | Supports `"MM"` and `"mm"` case aliases, skips None `leap` field | Passed |

