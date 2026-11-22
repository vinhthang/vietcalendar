# Vietnamese Luni-Solar Astronomical Calendar Engine

- Feature ID: 2026-11-21-astronomical-calendar
- Type: enhancement
- Status: In-Testing
- First entered In-Testing: 2026-11-21
- Related specifications: None

## Problem and source intent

Brownfield specification baseline requested by user: "Analyze this project, write AGENTS.md, write specs for existing features using my `spec-driven` skill... document current behavior and catalog discovered quirks/tech debt". Captures the existing astronomical calendar algorithms, lunar domain models, and service wrappers ported from Java (`VietCalendar.java`) to Rust.

## Repository grounding

- Legacy `src/calendar.rs` (migrated to `src/plugins/calendar_vn/`): `jd_from_date`, `jd_to_date`, `sun_longitude`, `sun_longitude_aa98`, `new_moon`, `new_moon_aa98`, `int`, `get_sun_longitude`, `get_new_moon_day`, `get_lunar_month_11`, `get_leap_month_offset`, `convert_solar_to_lunar`, `convert_lunar_to_solar`.
- Legacy `src/models.rs` (migrated to `src/contracts/models.rs`): `DateMonthYear { dd, mm, yyyy, leap }`, `LunarDate { day, month, year, is_leap }`.
- Legacy `src/services.rs` (migrated to `src/plugins/calendar_vn/`): `is_solar_leap`, `to_lunar`, `to_solar`.
- [`tests/integration_test.rs`](../../../tests/integration_test.rs): integration test suite.
- [`docs/ARCHITECTURE_DECISIONS.md`](../../ARCHITECTURE_DECISIONS.md): ADR-1 (`LunarDate` domain model) and ADR-2 (`is_leap_month` parameter in `to_solar`).

## Non-goals

- Modifying existing `.rs` source files or altering astronomical constants in this baseline specification pass.
- Validating Vietnam public holidays or compensatory leave (covered in [vietnam-holidays](../2026-11-21-vietnam-holidays/spec.md)).
- Changing `is_solar_leap` naming or `convert_lunar_to_solar` day-30 rollover behavior without a subsequent linked delta specification (`TD-03`, `TD-04` in [`docs/PROJECT_ANALYSIS.md`](../../PROJECT_ANALYSIS.md)).

## Requirements

### REQ-001: Julian Day Number Bidirectional Conversion

- Source intent: Baseline behavior in `src/calendar.rs::jd_from_date` and `jd_to_date` (Claus Tøndering algorithm) verified by `test_jd_roundtrip`.
- Statement: WHEN converting between Gregorian/Julian calendar dates `(dd, mm, yy)` and integer Julian Day Numbers, THE calendar module SHALL compute exact integer Julian Day Numbers using the Gregorian reform cutoff (`jd < 2299161` / `jd > 2299160` around October 1582) and round-trip back to `(day, month, year)`.
- AC-001-1 — Success: `jd_to_date(jd_from_date(d, m, y)) == (d, m, y)` holds across both Julian (`4/10/1582`) and Gregorian (`15/10/1582`, `29/2/2004`, `17/8/2026`) dates.
- AC-001-2 — Boundary/failure: Julian Day conversion preserves the October 1582 calendar transition boundary (`jd <= 2299160` routes through Julian offsets `32083`/`32082`; `jd >= 2299161` routes through Gregorian `32045`/`32044` century corrections).

### REQ-002: Jean Meeus AA98 Solar Longitude and New Moon Calculation

- Source intent: Baseline behavior in `src/calendar.rs::sun_longitude_aa98`, `new_moon_aa98`, `get_sun_longitude`, and `get_new_moon_day` derived from Jean Meeus *Astronomical Algorithms* (1998).
- Statement: WHEN evaluating solar longitude or lunar phase instants for a Julian date `jdn` or lunation index `k` and timezone offset `time_zone`, THE calendar module SHALL normalize solar longitude to `[0.0, 360.0)` degrees and localize new moon day indices using `int(jd + 0.5 + time_zone / 24.0)`.
- AC-002-1 — Success: `sun_longitude(jd)` returns values strictly in `[0.0, 360.0)`, and consecutive `new_moon(k)` values differ by approximately one synodic month (`~29.53` days, `|(nm1 - nm0) - 29.53| < 1.0`).
- AC-002-2 — Boundary/failure: `new_moon_aa98` switches the `deltat` polynomial adjustment at `t < -11.0` Julian centuries from 1900.0 so historical lunations before year ~800 do not reuse modern `deltat` coefficients.

### REQ-003: Bidirectional Solar and Vietnamese Lunar Conversion

- Current contract: Changed by [nonblocking-plugin-refactor/REQ-027](../2026-11-21-nonblocking-plugin-refactor/spec.md#req-027-moka-cached-lunarspan-engine-and-strict-lunar-day-validation)
- Source intent: Baseline behavior in `src/calendar.rs::convert_solar_to_lunar`, `convert_lunar_to_solar`, and `src/services.rs::to_lunar`, `to_solar` (guided by `ADR-1` and `ADR-2` in [`docs/ARCHITECTURE_DECISIONS.md`](../../ARCHITECTURE_DECISIONS.md)).
- Statement: WHEN converting a Gregorian `NaiveDate` to lunar or a lunar tuple `(day, month, year, is_leap_month, time_zone)` to solar, THE calendar and service layers SHALL anchor Lunar Month 11 to the winter solstice (`sun_long >= 9` sector adjustment), identify any leap month via `get_leap_month_offset`, represent lunar output via `models::LunarDate` (never `chrono::NaiveDate`), and reject non-leap-month or non-leap-year `is_leap_month = true` requests.
- AC-003-1 — Success: `to_lunar(2024-02-10, 7.0)` returns `LunarDate { day: 1, month: 1, year: 2024, is_leap: false }`; `to_solar(11, 2, 2004, true, 7.0)` returns `Some(2004-03-31)` while `to_solar(11, 2, 2004, false, 7.0)` returns `Some(2004-03-01)`; and 30-day lunar months (e.g. `2015-09-12` -> `30/7/2015`) never panic.
- AC-003-2 — Boundary/failure: IF `is_leap_month == true` is requested for a non-leap year (`b11 - a11 <= 365`) or for a month other than the actual leap month (`lunar_month != leap_month`), THEN `convert_lunar_to_solar` SHALL return `(0, 0, 0)` and `services::to_solar` SHALL map `(0, 0, 0)` to `None`.

### REQ-004: Metonic Cycle Predicate and Calendar DTO Serialization

- Current contract: Retired by [nonblocking-plugin-refactor/REQ-028](../2026-11-21-nonblocking-plugin-refactor/spec.md#req-028-clean-break-contracts-and-exact-is_lunar_leap_year-predicate)
- Source intent: Baseline behavior in `src/services.rs::is_solar_leap` and `src/models.rs::DateMonthYear`, `LunarDate`.
- Statement: WHEN checking Metonic leap-year cadence via `is_solar_leap(year)` or serializing/deserializing `DateMonthYear` and `LunarDate`, THE models and services modules SHALL evaluate `year % 19` against `{0, 3, 6, 9, 11, 14, 17}`, accept `"mm"` and `"MM"` JSON aliases on `DateMonthYear`, and omit `leap` during JSON serialization when `None`.
- AC-004-1 — Success: `is_solar_leap` returns `true` for `2004`, `2020`, `2023`, `2025` and `false` for `2005`, `2021`, `2022`, `2024`; `DateMonthYear` deserializes both `"MM"` and `"mm"` keys and omits `"leap"` when `None`.
- AC-004-2 — Boundary/failure: `LunarDate` round-trips cleanly through `serde_json` preserving `is_leap: bool`, and `is_solar_leap` uses pure modular residue without panicking on negative or zero `i32` inputs (noting documented `TD-04` naming quirk).

## Assumptions and owner-deferred questions

- **Baseline documentation assumption (reversible):** Current quirks `TD-03` (where `convert_lunar_to_solar` does not validate `lunar_month` in `1..=12` or reject day `30` on a 29-day *tháng thiếu*, causing rollover to the first day of the next lunar month) and `TD-04` (`is_solar_leap` naming for the Metonic lunar cycle) are documented as-is under [`docs/PROJECT_ANALYSIS.md`](../../PROJECT_ANALYSIS.md); fixing them requires a future Case 3 linked delta package.
- **No owner-deferred questions:** All astronomical baseline behavior was settled by inspection of existing code and user instructions ("document current behavior and catalog discovered quirks/tech debt").

## Approvals and lifecycle history

- 2026-11-21 (`Planning -> In-Progress`): Authorized via `/goal` and settled plan answers ("Advance all 5 baseline specs to In-Testing with real execution evidence") covering `REQ-001..REQ-004` after Gate 1 and Gate 2 passed.
- 2026-11-21 (`In-Progress -> In-Testing`): [Task 1](./tasks.md#task-1) and [Run 1](./verification.md#run-1--2026-11-21) passed; awaiting human confirmation on real-world usage before promoting to `Verified`.
