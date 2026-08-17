# VietCalendar Project Guidelines for Antigravity AI

This file contains primary directives, architectural guidelines, and context for Antigravity AI agents working in this repository.

---

## 1. Mandatory First Step: Read the Documentation

Before performing any code modifications, refactoring, or feature development, **you MUST inspect the documentation in the [`docs/`](docs/) directory**:

* **[`docs/ARCHITECTURE_DECISIONS.md`](docs/ARCHITECTURE_DECISIONS.md)** (⭐ **Most Important**):
  Comprehensive Architecture Decision Record (ADR). Details the domain models, Jean Meeus astronomical algorithms, leap month vs. leap year handling, error mapping (`AppError`), timezone anchoring (UTC+7), and Vietnam Labor Code holiday & compensatory leave rules.
* **[`docs/implementation_plan.md`](docs/implementation_plan.md)**:
  Initial architectural blueprint for the Java 11 / Maven Vert.x to Rust Axum migration.
* **[`docs/implementation_file_2.md`](docs/implementation_file_2.md)**:
  Observability setup, OpenTelemetry tracing, Utoipa Swagger/OpenAPI specification, and CI/CD automation.

---

## 2. Core Domain Invariants & Rules

1. **Lunar Date Representation:**
   * **NEVER** use `chrono::NaiveDate` to store or return lunar dates. `NaiveDate` enforces Gregorian constraints and will panic on lunar months with 30 days in February (e.g. tháng 2 đủ 30 ngày).
   * Always use `models::LunarDate` (`{ day: i32, month: i32, year: i32, is_leap: bool }`).

2. **Leap Month (`is_leap_month`) in `to_solar`:**
   * In the Vietnamese Luni-Solar calendar, a leap year (năm nhuận) has one specific leap month (tháng nhuận).
   * In `to_solar(day, month, year, is_leap_month, time_zone)`, pass `is_leap_month = true` **only** when converting a date within that specific leap month, not for all dates in a leap year.

3. **HTTP API Error Handling:**
   * Handlers must never return `Result<Json<T>, String>` directly, as Axum serializes `String` errors with `HTTP 200 OK`.
   * Return `Result<Json<T>, AppError>` which maps to `HTTP 400 Bad Request` with `{ "error": "<message>" }` JSON payload.

4. **Timezone Anchoring:**
   * The `home()` endpoint and date operations must calculate the current date anchored to Vietnam Indochina Time: `(chrono::Utc::now() + chrono::Duration::hours(7)).date_naive()`.

5. **Vietnam Public Holidays & Compensatory Leave:**
   * Saturday and Sunday are holidays.
   * Fixed solar holidays: `01/01`, `30/04`, `01/05`, `02/09`.
   * Lunar holidays: `10/03` Lunar (Giỗ Tổ Hùng Vương) and Tết Nguyên Đán (Eve + Lunar `01/01`, `02/01`, `03/01`).
   * Compensatory rule (Nghỉ bù): When a public holiday falls on Saturday or Sunday, compensatory holiday(s) are granted on the next available working weekday(s).

---

## 3. Technology Stack & Commands

* **Language & Framework:** Rust 1.94, Axum 0.8, Tokio 1.43, Chrono 0.4, Utoipa 5.4.
* **Run Tests:** `cargo test`
* **Format & Lint:** `cargo fmt --all -- --check` and `cargo clippy -- -D warnings`
* **Run Locally:** `cargo run` (default port `8080` or `$PORT` / `$HTTP_PORT`)
* **Docker Build:** `docker build -t vietcalendar-rs .`

---

## 4. Breaking Changes & Architectural Decisions Protocol

Whenever introducing a **breaking change** (e.g. changing endpoint routes, request/response contracts, modifying domain models, or altering business invariants):

1. **Document in [`docs/ARCHITECTURE_DECISIONS.md`](docs/ARCHITECTURE_DECISIONS.md):**
   * Immediately record a new Architecture Decision Record (ADR) detailing **Context**, **Decision**, **Rationale**, and **Verification**.
   * Keep the architecture diagram (`mermaid`) and the **Verification Matrix** updated.

2. **Synchronize [`README.md`](README.md):**
   * Keep all endpoint descriptions, parameter tables, sample requests, and responses 100% up-to-date with code changes.

3. **Synchronize OpenAPI / Swagger (`utoipa`):**
   * Update `#[utoipa::path(...)]` attributes on handlers and register paths/schemas in `ApiDoc` inside `src/main.rs`.

4. **Add & Update Unit & Integration Tests:**
   * Never modify an API contract without writing corresponding unit tests in `src/handlers.rs` (or `src/services.rs`) and verifying with `cargo test`.

5. **Highlight in User Communication:**
   * Explicitly flag breaking changes and migrations to the user with GitHub warning alerts (`> [!WARNING]`).

