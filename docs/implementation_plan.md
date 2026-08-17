# Convert Maven Vert.x Project to Rust

This project will be converted from a Java 11 Maven Vert.x application to a modern, high-performance Rust web application using `axum` and `tokio`.

## Proposed Changes

We will use the following Rust stack:
- **Web Framework**: `axum` (extremely fast, ergonomic, built on Tokio)
- **Async Runtime**: `tokio`
- **Serialization**: `serde` and `serde_json`
- **DateTime**: `chrono`

### Application Structure

The new project will be created in `/home/thang/vietcalendar-rs`. The existing Java project in `/home/thang/vietcalendar` will be kept intact as a reference.

#### [NEW] `Cargo.toml`
Initialize dependencies and project metadata.

#### [NEW] `Dockerfile`
A Dockerfile for building and deploying the Rust web server as a lightweight container (updated to use the local Rust container image `rust:1.94`).

#### [NEW] `src/main.rs`
Entry point for the application. Sets up the Axum router with the following endpoints (matching the Vert.x ones):
- `GET /` -> Returns today's lunar date
- `GET /lunar` -> Accepts `dd`, `MM`, `yyyy`, `timeZone` query parameters and returns Lunar date
- `GET /lunar/:ddMMyyyy` -> Accepts path param and returns Lunar date
- `GET /vietnam-holiday` -> Accepts `dd`, `MM`, `yyyy` query parameters and returns boolean

*Note on Configuration*: 
- **HTTP Port**: The server will listen on a custom HTTP port, configurable via the `PORT` or `HTTP_PORT` environment variable (falling back to `8080` if absent).
- **Worker Threads**: The number of worker threads for Axum (Tokio) can be configured seamlessly via the `TOKIO_WORKER_THREADS` environment variable because we will rely on Tokio's multi-thread runtime scheduler. If not set, it defaults to the number of available CPU cores on the machine.

#### [NEW] `src/calendar.rs`
A direct Rust port of `de.unileipzig.informatik.VietCalendar.java`. This contains the core astronomical Julian to Lunar/Solar math algorithms. **Per user request, all internal implementation comments/javadocs from the original Java file were preserved and added into the Rust port.**

#### [NEW] `src/services.rs`
Ports of `VietCalendarService.java`, `LunarCalendarService.java`, and `VietNamNationalHolidayService.java`.
- `to_lunar` conversions
- `is_vietnam_holiday` logic including static solar/lunar holiday lists.

#### [NEW] `src/models.rs`
Rust `struct`s replacing `DateMonthYear` and other DTOs, deriving `serde::Serialize` and `serde::Deserialize`.

#### [NEW] `src/handlers.rs`
Axum Request Handlers.

---

## Verification Plan

### Automated Tests
I will port the existing JUnit tests into Rust's built-in testing framework:
1. `LunarCalendarServiceTest.java` -> `src/services/tests.rs` (Tests lunar/solar conversions and leap years).
2. `VietnamNationalHolidayTest.java` -> `src/services/holiday_tests.rs` (Tests holiday validation).
3. We will run `cargo test` to ensure the algorithms correctly compute Lunar dates exactly as the Java version did.

### Manual Verification
1. Run `cargo run`.
2. Send HTTP requests (via `curl`) to `http://localhost:8080/lunar?dd=12&MM=09&yyyy=2015` and verify the JSON output matches `{ "dd": 30, "MM": 7, "yyyy": 2015 }`.
3. Send requests to `/vietnam-holiday` to verify holiday logic.
