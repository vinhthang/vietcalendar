# Verification: Vietnam Public Holidays and Compensatory Leave Engine

Governing specification: [spec.md](./spec.md)

## Planned checks

### AC-005-1 (REQ-005)

- Method: Unit test `services::tests::test_solar_holiday_check` and integration test `test_holiday` (`2015-05-03` Sunday, `2015-04-30`).
- Expected outcome: Fixed dates (`01/01`, `30/04`, `01/05`, `02/09`) and weekend dates return `true`.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-005-2 (REQ-005)

- Method: Unit test `services::tests::test_solar_holiday_check` (`2024-05-15` Wednesday) and integration test `test_holiday` (`2015-05-04` Monday).
- Expected outcome: Non-holiday weekdays return `false`.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-006-1 (REQ-006)

- Method: Unit test `services::tests::test_vietnam_holidays_with_compensatory` checking `2024-02-09` (Tết Eve) and `2024-02-10` (Tết Day 1).
- Expected outcome: Both `is_vietnam_holiday(2024-02-09, 7.0)` and `is_vietnam_holiday(2024-02-10, 7.0)` return `true`.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-006-2 (REQ-006)

- Method: Source inspection of `src/services.rs::get_all_holidays_for_year` (`L99-L127`).
- Expected outcome: Lunar years `solar_year - 1..=solar_year + 1` are queried, Eve is computed as `tet1 - Duration::days(1)`, and `.year() == solar_year` filters cross-year dates.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-007-1 (REQ-007)

- Method: Unit test `services::tests::test_vietnam_holidays_with_compensatory` and integration test `test_holiday` on `2011-04-30`..`2011-05-04`.
- Expected outcome: `2011-04-30`, `2011-05-01`, `2011-05-02`, and `2011-05-03` return `true`, while `2011-05-04` returns `false`.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-007-2 (REQ-007)

- Method: Source inspection of `src/services.rs:9,53-74` and repeated calls across `2011` and `2024` in `test_vietnam_holidays_with_compensatory`.
- Expected outcome: `HOLIDAY_CACHE` caches `HashSet<NaiveDate>` per `year` under `OnceLock<RwLock<HashMap<i32, HashSet<NaiveDate>>>>` and serves subsequent lookups via read lock.
- Execution: [Run 1](#run-1--2026-11-21)

## Execution evidence

### Run 1 — 2026-11-21

- Checked: REQ-005, REQ-006, REQ-007 / AC-005-1..AC-007-2 (commit `e95a3d4`, `rustc 1.92.0` / `cargo 1.92.0`)
- Command: `rtk proxy cargo test --offline` (exit code: `0`) + static inspection of `src/services.rs`
- Output excerpt:
  ```text
  running 23 tests
  ...
  test services::tests::test_solar_holiday_check ... ok
  test services::tests::test_vietnam_holidays_with_compensatory ... ok
  test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

  Running tests/integration_test.rs
  running 6 tests
  ...
  test test_holiday ... ok
  test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
  ```
- Result: AC-005-1 `PASS`, AC-005-2 `PASS`, AC-006-1 `PASS`, AC-006-2 `PASS`, AC-007-1 `PASS`, AC-007-2 `PASS` — holiday unit & integration tests passed and `HOLIDAY_CACHE`/year-boundary code inspected.

## Real-world observations

No real-world confirmation has been recorded.
