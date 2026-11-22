# Decisions: Vietnam Public Holidays and Compensatory Leave Engine

Governing specification: [spec.md](./spec.md)

## Architectural rationale

- Governs: REQ-005, REQ-006, REQ-007

Vietnam holiday detection (`src/services.rs`) builds on the astronomical conversion primitives from [astronomical-calendar](../2026-11-21-astronomical-calendar/spec.md#req-003-bidirectional-solar-and-vietnamese-lunar-conversion) so any historical or future solar year can be checked without hardcoded multi-year datasets. See `ADR-002` for the dynamic computation and `HOLIDAY_CACHE` design.

## ADR-002: Dynamic Holiday and Compensatory Calculation with Process-Wide Cache

- Governs: REQ-005, REQ-006, REQ-007
- State: Accepted
- Context and constraints: Vietnam public holidays combine fixed Gregorian dates with lunar holidays (`10/03` Lunar, Tết Eve through Mùng 3) and sequential weekend compensatory leave (*nghỉ bù*), where chained holidays (e.g. `30/04` Sat + `01/05` Sun) shift forward past one another onto subsequent working weekdays. Recomputing astronomical conversions for every HTTP or MCP request under load (`load_test.js`) adds avoidable latency.
- Viable options: Option A — Compute each year's `HashSet<NaiveDate>` dynamically via `get_all_holidays_for_year` (including weekend rollover) and cache results in a `static OnceLock<RwLock<HashMap<i32, HashSet<NaiveDate>>>>`; vs. Option B — Maintain static precomputed JSON/Rust holiday lookup tables per year.
- Decision: Option A — Dynamic computation in `get_all_holidays_for_year` backed by `HOLIDAY_CACHE`.
- Costs and risks: Supports arbitrary years in `O(1)` amortized lookup after first access and naturally computes multi-day compensatory chains (`2011-04-30`/`05-01` -> `05-02`/`05-03`). Known trade-offs in current baseline (`TD-02`, `TD-05`, `TD-07`): cache keys omit `time_zone`, `.read().unwrap()` / `.write().unwrap()` panic if poisoned, and Labor Code 2019 updates are not yet incorporated.
- Rejected alternatives: Option B requires ongoing manual data maintenance for every future year and risks omission/transcription errors on lunar dates.
- Affected components: `src/services.rs` (`HOLIDAY_CACHE`, `is_vietnam_holiday`, `is_solar_holiday`, `get_all_holidays_for_year`).

## Gate 1: normative review

| Check | Status | Evidence / required correction |
| --- | --- | --- |
| Codebase grounding | PASS | Grounded in `src/services.rs`, `tests/integration_test.rs::test_holiday`, and `docs/ARCHITECTURE_DECISIONS.md` (`ADR-5`). |
| Attributable intent and explicit scope | PASS | `REQ-005`..`REQ-007` trace to brownfield audit and `ADR-5`; Labor Code 2019 expansion and `TD-01`/`TD-02` fixes excluded in `Non-goals`. |
| Observable requirements and meaningful acceptance criteria | PASS | `AC-005-1`..`AC-007-2` specify concrete dates (`2011-04-30`..`2011-05-04`, `2024-02-09`..`2024-02-10`, `2024-05-15`) and cache behavior. |
| Assumptions and owner-deferred questions are explicit | PASS | `TD-05` pre-2021 holiday rule and `TD-02` cache key assumption recorded explicitly; zero `[PROVISIONAL]` or `TODO` items. |
| Architectural rationale respects requirements | PASS | `ADR-002` governs `REQ-005`..`REQ-007` using stdlib synchronization primitives already present in `src/services.rs`. |
| IDs, references, and revision route are consistent | PASS | `REQ-005..REQ-007` and `ADR-002` match `docs/specs/INDEX.md`; cross-spec link to `REQ-003` valid. |
