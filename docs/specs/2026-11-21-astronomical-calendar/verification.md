# Verification: Vietnamese Luni-Solar Astronomical Calendar Engine

Governing specification: [spec.md](./spec.md)

## Planned checks

### AC-001-1 (REQ-001)

- Method: Unit test `calendar::tests::test_jd_roundtrip` in `src/calendar.rs`.
- Expected outcome: Roundtrip `(d, m, y) -> jd_from_date -> jd_to_date` succeeds for Julian (`4/10/1582`) and Gregorian (`15/10/1582`, `29/2/2004`, `17/8/2026`, etc.) dates.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-001-2 (REQ-001)

- Method: Unit test `calendar::tests::test_jd_roundtrip` combined with source inspection of `src/calendar.rs:15-17,32-40`.
- Expected outcome: `jd < 2299161` (`jd_from_date`) and `jd > 2299160` (`jd_to_date`) branch between Julian and Gregorian century adjustments around `4/10/1582` and `15/10/1582`.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-002-1 (REQ-002)

- Method: Unit tests `calendar::tests::test_sun_longitude_range` and `calendar::tests::test_new_moon_calculation` in `src/calendar.rs`.
- Expected outcome: `sun_longitude` outputs lie within `[0.0, 360.0)` and `new_moon(1) - new_moon(0)` stays within `1.0` day of `29.53`.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-002-2 (REQ-002)

- Method: Source inspection of `src/calendar.rs::new_moon_aa98` (`L103-L108`).
- Expected outcome: `deltat` switches polynomial formula at `t < -11.0`.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-003-1 (REQ-003)

- Method: Unit tests `calendar::tests::test_solar_to_lunar_and_back`, `services::tests::test_to_lunar_and_to_solar`, and integration tests `test_to_lunar`, `test_to_lunar2`, `test_to_solar`, `test_roundtrip_conversion` in `tests/integration_test.rs`.
- Expected outcome: Exact solar/lunar conversions (`2024-02-10 <-> 1/1/2024`, `2004-03-31 <-> 11/2/2004 leap`, `2004-03-01 <-> 11/2/2004 non-leap`, `2015-09-12 -> 30/7/2015`) pass without panics.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-003-2 (REQ-003)

- Method: Unit test `services::tests::test_to_solar_invalid_leap_month` and source inspection of `src/calendar.rs:219-234` and `src/services.rs:39-44`.
- Expected outcome: `to_solar(1, 1, 2024, true, 7.0)` returns `None`, matching `(0, 0, 0)` rejection in `convert_lunar_to_solar`.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-004-1 (REQ-004)

- Method: Unit tests `services::tests::test_metonic_leap_years`, `models::tests::test_date_month_year_serde`, and integration test `test_leap`.
- Expected outcome: Metonic cadence assertions (`2004`, `2020`, `2023`, `2025` true; `2005`, `2021`, `2022`, `2024` false) and `DateMonthYear` `"MM"`/`"mm"`/`leap` serialization assertions pass.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-004-2 (REQ-004)

- Method: Unit test `models::tests::test_lunar_date_serde` and source inspection of `src/services.rs:11-14`.
- Expected outcome: `LunarDate` JSON roundtrips identically and `is_solar_leap` evaluates `year % 19` safely.
- Execution: [Run 1](#run-1--2026-11-21)

## Execution evidence

### Run 1 — 2026-11-21

- Checked: REQ-001, REQ-002, REQ-003, REQ-004 / AC-001-1..AC-004-2 (commit `e95a3d4`, `rustc 1.92.0` / `cargo 1.92.0`)
- Command: `rtk proxy cargo test --offline` (exit code: `0`) + static inspection of `src/calendar.rs`, `src/models.rs`, `src/services.rs`
- Output excerpt:
  ```text
  running 23 tests
  test calendar::tests::test_jd_roundtrip ... ok
  test calendar::tests::test_solar_to_lunar_and_back ... ok
  test calendar::tests::test_new_moon_calculation ... ok
  test calendar::tests::test_sun_longitude_range ... ok
  ...
  test models::tests::test_date_month_year_serde ... ok
  test models::tests::test_lunar_date_serde ... ok
  test services::tests::test_to_lunar_and_to_solar ... ok
  test services::tests::test_to_solar_invalid_leap_month ... ok
  test services::tests::test_metonic_leap_years ... ok
  test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

  Running tests/integration_test.rs
  running 6 tests
  test test_leap ... ok
  test test_roundtrip_conversion ... ok
  test test_to_lunar2 ... ok
  test test_to_lunar ... ok
  test test_to_solar ... ok
  test test_holiday ... ok
  test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
  ```
- Result: AC-001-1 `PASS`, AC-001-2 `PASS`, AC-002-1 `PASS`, AC-002-2 `PASS`, AC-003-1 `PASS`, AC-003-2 `PASS`, AC-004-1 `PASS`, AC-004-2 `PASS` — all 14 astronomical/model unit & integration tests passed; `deltat` `t < -11.0` and `1582` cutoff verified by source inspection (`cargo fmt`/`clippy` unavailable on local RPM toolchain).

## Real-world observations

No real-world confirmation has been recorded.
