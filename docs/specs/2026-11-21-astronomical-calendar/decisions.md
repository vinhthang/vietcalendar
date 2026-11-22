# Decisions: Vietnamese Luni-Solar Astronomical Calendar Engine

Governing specification: [spec.md](./spec.md)

## Architectural rationale

- Governs: REQ-001, REQ-002, REQ-003, REQ-004

The astronomical layer (`src/calendar.rs`), domain models (`src/models.rs`), and conversion wrappers (`src/services.rs`) encapsulate Jean Meeus 1998 lunar/solar calculations and Claus Tøndering Julian Day arithmetic without external astronomical libraries or persistent lookup files. See `ADR-001` below for the core model and computation strategy; `REQ-001` and `REQ-002` preserve the deterministic pure-function algorithms from the Java reference port (`VietCalendar.java`), while `REQ-004` preserves existing wire compatibility (`DateMonthYear` `"mm"`/`"MM"` aliases and `is_solar_leap` signature) for backwards compatibility with legacy clients.

## ADR-001: Dedicated `LunarDate` Domain Model and Pure-Rust Astronomical Computation

- Governs: REQ-001, REQ-002, REQ-003, REQ-004
- State: Accepted
- Context and constraints: Lunar months are synodic (29 or 30 days) and include leap months (tháng nhuận) whose placement depends on true solar longitude vs. new-moon timing in a specific timezone (e.g., Vietnam UTC+7). Storing lunar dates in `chrono::NaiveDate` previously caused panics on 30-day lunar month 2, and passing `leap = 1` for all months in a leap year broke non-leap conversions.
- Viable options: Option A — Dedicated `models::LunarDate { day, month, year, is_leap }` paired with runtime Jean Meeus AA98 astronomical math (`src/calendar.rs`); vs. Option B — Bundling static lookup tables for a bounded year range (e.g., 1900–2100) using a third-party calendar crate or embedded JSON.
- Decision: Option A — Dedicated `models::LunarDate` paired with pure-Rust Jean Meeus AA98 astronomical math and `is_leap_month`-scoped `to_solar`.
- Costs and risks: Eliminates bounded-century lookup tables, zero external astronomical crate dependencies, and prevents Gregorian calendar validation panics (`ADR-1`, `ADR-2`). Existing caveats `TD-03` (missing 29-day month length check in `convert_lunar_to_solar`) and `TD-04` (`is_solar_leap` Metonic naming) remain in baseline code and are cataloged for follow-up delta specs.
- Rejected alternatives: Option B restricts historical/future dates to the embedded table horizon, increases binary footprint, and cannot dynamically shift month boundaries for non-`7.0` timezones.
- Affected components: `src/calendar.rs`, `src/models.rs`, `src/services.rs`.

## Gate 1: normative review

| Check | Status | Evidence / required correction |
| --- | --- | --- |
| Codebase grounding | PASS | Grounded in `src/calendar.rs`, `src/models.rs`, `src/services.rs`, `tests/integration_test.rs`, and `docs/ARCHITECTURE_DECISIONS.md` (ADR-1, ADR-2). |
| Attributable intent and explicit scope | PASS | Every `REQ-001`..`REQ-004` traces to brownfield audit intent and verified baseline symbols; holidays and code changes excluded in `Non-goals`. |
| Observable requirements and meaningful acceptance criteria | PASS | `AC-001-1`..`AC-004-2` define concrete dates, bounds (`[0.0, 360.0)`), `Some`/`None` outputs, and Serde JSON roundtrip outcomes. |
| Assumptions and owner-deferred questions are explicit | PASS | Baseline documentation assumptions (`TD-03`, `TD-04`) documented explicitly; zero `TODO` or `[PROVISIONAL]` markers. |
| Architectural rationale respects requirements | PASS | `ADR-001` and `## Architectural rationale` govern `REQ-001`..`REQ-004` without mutating `.rs` code. |
| IDs, references, and revision route are consistent | PASS | `REQ-001..REQ-004` and `ADR-001` match `docs/specs/INDEX.md`; `First entered In-Testing: Never`. |
