# Verification: HTTP REST API and OpenAPI Specification

Governing specification: [spec.md](./spec.md)

## Planned checks

### AC-008-1 (REQ-008)

- Method: Source inspection of `src/handlers.rs::home` (`L77-L86`).
- Expected outcome: Computes `(Utc::now() + chrono::Duration::hours(7)).date_naive()`, converts with `to_lunar(..., 7.0)`, and populates `leap: Some(true)` iff `is_leap`.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-008-2 (REQ-008)

- Method: Source inspection of `src/handlers.rs::home` signature (`L77`).
- Expected outcome: Takes zero request parameters and returns `Json<DateMonthYear>` directly without error branches.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-009-1 (REQ-009)

- Method: Async unit test `handlers::tests::test_get_lunar_handler` and `models::tests::test_date_month_year_serde` / `LunarQuery` alias inspection (`src/handlers.rs:38-43`).
- Expected outcome: `get_lunar` returns `30/7/2015` for `12/9/2015` and supports `"mm"`/`"MM"` and `"timeZone"`/`"timezone"`.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-009-2 (REQ-009)

- Method: Async unit test `handlers::tests::test_get_lunar_handler_invalid_date` (`31/02/2024`).
- Expected outcome: `get_lunar` returns `Err(AppError::BadRequest)` containing `"Invalid solar date"`.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-010-1 (REQ-010)

- Method: Async unit tests `handlers::tests::test_get_solar_to_lunar_endpoint` and `handlers::tests::test_get_lunar_to_solar_endpoint`.
- Expected outcome: `2015-09-12` converts to `30/7/2015`, `2015-07-30` converts to `12/9/2015`, and `2004-02-11` (`leap=true`) converts to `31/3/2004`.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-010-2 (REQ-010)

- Method: Async unit tests `test_get_solar_to_lunar_endpoint` (`"12-09-2015"`) and `test_get_lunar_to_solar_endpoint` (`"20150730"`), plus source inspection of `src/handlers.rs:161-191`.
- Expected outcome: Malformed ISO dates, out-of-range `mm`/`dd`, and invalid `to_solar` results return `AppError::BadRequest`.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-011-1 (REQ-011)

- Method: Async unit test `handlers::tests::test_check_vietnam_holiday_handler` (`30/4/2015`).
- Expected outcome: `check_vietnam_holiday` returns `Json(true)`.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-011-2 (REQ-011)

- Method: Source inspection of `src/handlers.rs::check_vietnam_holiday` (`L210-L216`).
- Expected outcome: Invalid `(yyyy, mm, dd)` triggers `AppError::BadRequest("Invalid date: ...")`.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-012-1 (REQ-012)

- Method: Async unit test `handlers::tests::test_app_error_into_response` and source inspection of `src/main.rs:21-37,115-141`.
- Expected outcome: `AppError::BadRequest` produces `400`, `AppError::NotFound` produces `404`, `ApiDoc` registers all 5 handlers + 3 schemas under `SwaggerUi`, and `CorsLayer` allows `Any`.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-012-2 (REQ-012)

- Method: Source inspection across `src/handlers.rs` (`L77-L216`).
- Expected outcome: Every fallible handler returns `Result<Json<T>, AppError>` (zero `Result<Json<T>, String>` signatures).
- Execution: [Run 1](#run-1--2026-11-21)

## Execution evidence

### Run 1 — 2026-11-21

- Checked: REQ-008, REQ-009, REQ-010, REQ-011, REQ-012 / AC-008-1..AC-012-2 (commit `e95a3d4`, `rustc 1.92.0` / `cargo 1.92.0`)
- Command: `rtk proxy cargo test --offline` (exit code: `0`) + static inspection of `src/handlers.rs` and `src/main.rs`
- Output excerpt:
  ```text
  running 23 tests
  ...
  test handlers::tests::test_get_lunar_handler ... ok
  test handlers::tests::test_get_lunar_handler_invalid_date ... ok
  test handlers::tests::test_check_vietnam_holiday_handler ... ok
  test handlers::tests::test_get_solar_to_lunar_endpoint ... ok
  test handlers::tests::test_get_lunar_to_solar_endpoint ... ok
  ...
  test handlers::tests::test_app_error_into_response ... ok
  test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
  ```
- Result: AC-008-1 `PASS`, AC-008-2 `PASS`, AC-009-1 `PASS`, AC-009-2 `PASS`, AC-010-1 `PASS`, AC-010-2 `PASS`, AC-011-1 `PASS`, AC-011-2 `PASS`, AC-012-1 `PASS`, AC-012-2 `PASS` — all 6 handler tests passed; `home()`, `check_vietnam_holiday`, `ApiDoc`, and `CorsLayer` verified by code inspection (full router `oneshot` HTTP tests remain a documented coverage gap).

## Real-world observations

No real-world confirmation has been recorded.
