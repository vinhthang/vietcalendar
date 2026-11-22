# Vietnam Public Holidays and Compensatory Leave Engine

- Feature ID: 2026-11-21-vietnam-holidays
- Type: enhancement
- Status: In-Testing
- First entered In-Testing: 2026-11-21
- Related specifications: None

## Problem and source intent

Brownfield specification baseline requested by user: "Analyze this project, write AGENTS.md, write specs for existing features using my `spec-driven` skill... document current behavior and catalog discovered quirks/tech debt". Captures the existing Vietnam holiday rules, weekend compensatory leave (*nghỉ bù*), and in-memory caching implemented in `src/services.rs`.

## Repository grounding

- Legacy `src/services.rs` (migrated to `src/plugins/holidays_vn/`): `HOLIDAY_CACHE`, `is_vietnam_holiday`, `is_solar_holiday`, `get_all_holidays_for_year`.
- [`tests/integration_test.rs`](../../../tests/integration_test.rs): integration test suite.
- [`docs/ARCHITECTURE_DECISIONS.md`](../../ARCHITECTURE_DECISIONS.md): ADR-5 (Compensatory Leave and Lunar New Year Eve Calculation).

## Non-goals

- Implementing post-2021 Vietnam Labor Code 2019 extensions (5-day Tết and 2-day National Day) without owner confirmation (`TD-05`).
- Fixing `HOLIDAY_CACHE` missing timezone key (`TD-02`) or `src/mcp.rs::compute_holiday_list_for_year` divergence (`TD-01`) during this documentation-only pass.

## Requirements

### REQ-005: Weekend and Fixed Solar Holiday Classification

- Current contract: Changed by [nonblocking-plugin-refactor/REQ-029](../2026-11-21-nonblocking-plugin-refactor/spec.md#req-029-data-driven-per-year-vietnam-holiday-plugin-working_weekends-and-fallback)
- Source intent: Baseline behavior in `src/services.rs::is_solar_holiday` and `is_vietnam_holiday` verified by `test_solar_holiday_check`.
- Statement: WHEN evaluating whether a solar `NaiveDate` is a weekend or fixed Vietnam solar holiday, THE services module SHALL treat Saturday, Sunday, `01/01` (New Year's Day), `30/04` (Reunification Day), `01/05` (International Workers' Day), and `02/09` (National Day) as holidays.
- AC-005-1 — Success: `is_solar_holiday` and `is_vietnam_holiday` return `true` for `01/01`, `30/04`, `01/05`, `02/09`, and any `Saturday` or `Sunday`.
- AC-005-2 — Boundary/failure: `is_solar_holiday` returns `false` for non-holiday weekdays such as `2024-05-15` (Wednesday).

### REQ-006: Lunar Holidays Across Solar-Year Boundaries

- Current contract: Changed by [nonblocking-plugin-refactor/REQ-029](../2026-11-21-nonblocking-plugin-refactor/spec.md#req-029-data-driven-per-year-vietnam-holiday-plugin-working_weekends-and-fallback)
- Source intent: Baseline behavior in `src/services.rs::get_all_holidays_for_year` and `ADR-5` in [`docs/ARCHITECTURE_DECISIONS.md`](../../ARCHITECTURE_DECISIONS.md).
- Statement: WHEN assembling lunar public holidays for a target `solar_year`, THE holiday engine SHALL query lunar years `solar_year - 1`, `solar_year`, and `solar_year + 1` via `to_solar(..., false, time_zone)` to include Giỗ Tổ Hùng Vương (`10/03` Lunar), Tết Eve (`to_solar(1, 1, ly, false, tz) - 1 day`), and Tết days `01/01`, `02/01`, and `03/01` Lunar whose solar dates fall within `solar_year`.
- AC-006-1 — Success: For solar year `2024`, both Tết Eve (`2024-02-09`, Friday) and Tết Day 1 (`2024-02-10`, Saturday) evaluate as holidays under `is_vietnam_holiday`.
- AC-006-2 — Boundary/failure: Subtracting `Duration::days(1)` from `to_solar(1, 1, ly)` handles both 29-day (*tháng thiếu*) and 30-day (*tháng đủ*) 12th lunar months and filters out lunar holidays whose mapped solar date falls outside `solar_year`.

### REQ-007: Weekend Compensatory Leave Rollover and Yearly Cache

- Current contract: Changed by [nonblocking-plugin-refactor/REQ-029](../2026-11-21-nonblocking-plugin-refactor/spec.md#req-029-data-driven-per-year-vietnam-holiday-plugin-working_weekends-and-fallback)
- Source intent: Baseline behavior in `src/services.rs::is_vietnam_holiday` and `get_all_holidays_for_year` verified by `test_vietnam_holidays_with_compensatory` and `test_holiday`.
- Statement: WHEN a public holiday in `get_all_holidays_for_year` falls on Saturday or Sunday, THE holiday engine SHALL add a compensatory holiday on the earliest subsequent date that is neither a weekend nor already in the holiday set, and memoize the computed `HashSet<NaiveDate>` per `solar_year` in `HOLIDAY_CACHE`.
- AC-007-1 — Success: In `2011`, consecutive weekend holidays `2011-04-30` (Sat) and `2011-05-01` (Sun) generate compensatory holidays on `2011-05-02` (Mon) and `2011-05-03` (Tue), while `2011-05-04` (Wed) returns `false`.
- AC-007-2 — Boundary/failure: Repeated calls to `is_vietnam_holiday` for the same `solar_year` reuse the `OnceLock<RwLock<HashMap<i32, HashSet<NaiveDate>>>>` entry in `HOLIDAY_CACHE` without recomputing `get_all_holidays_for_year` (noting documented `TD-02` timezone-key limitation).

## Assumptions and owner-deferred questions

- **Legal rule baseline (`TD-05`):** Current implementation reflects the 4-day Tết (Eve + Mùng 1–3) and 1-day National Day (`02/09`) rules rather than Vietnam Labor Code 2019 (effective 2021: 5 Tết days + 2 National Days). Any update requires owner decision and a Case 3 linked delta spec.
- **Cache key quirk (`TD-02`):** `HOLIDAY_CACHE` currently keys solely by `i32` `solar_year` and ignores `time_zone`; documented as baseline tech debt in [`docs/PROJECT_ANALYSIS.md`](../../PROJECT_ANALYSIS.md).

## Approvals and lifecycle history

- 2026-11-21 (`Planning -> In-Progress`): Authorized via `/goal` and settled plan answers ("Advance all 5 baseline specs to In-Testing with real execution evidence") covering `REQ-005..REQ-007` after Gate 1 and Gate 2 passed.
- 2026-11-21 (`In-Progress -> In-Testing`): [Task 1](./tasks.md#task-1) and [Run 1](./verification.md#run-1--2026-11-21) passed; awaiting human confirmation on real-world usage before promoting to `Verified`.
