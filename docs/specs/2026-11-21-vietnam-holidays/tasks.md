# Tasks: Vietnam Public Holidays and Compensatory Leave Engine

Sources: [spec.md](./spec.md), [design.md](./design.md)

## Task 1

**Verify Vietnam holiday engine baseline and tests**

- [x] Complete the work and its applicable checks.
- Covers: REQ-005, REQ-006, REQ-007 / AC-005-1, AC-005-2, AC-006-1, AC-006-2, AC-007-1, AC-007-2
- Files: `docs/specs/2026-11-21-vietnam-holidays/spec.md`, `docs/specs/2026-11-21-vietnam-holidays/decisions.md`, `docs/specs/2026-11-21-vietnam-holidays/design.md`, `docs/specs/2026-11-21-vietnam-holidays/tasks.md`, `docs/specs/2026-11-21-vietnam-holidays/verification.md`
- Dependencies: None
- Blocked by: None
- Work: Audit `src/services.rs` and `tests/integration_test.rs::test_holiday`; verify all holiday unit/integration tests pass via `cargo test --offline`; record `TD-02` and `TD-05` quirks without editing `.rs` code.
- Completion evidence: `cargo test --offline` passed (`test_solar_holiday_check`, `test_vietnam_holidays_with_compensatory`, `test_holiday`) plus source verification recorded in [Run 1](./verification.md#run-1--2026-11-21).

## Gate 2: traceability and readiness

| Requirement | AC | ADR or rationale | Design | Task | Planned verification | Status / reason |
| --- | --- | --- | --- | --- | --- | --- |
| REQ-005 | AC-005-1 | [ADR-002](./decisions.md#adr-002-dynamic-holiday-and-compensatory-calculation-with-process-wide-cache) | [Components](./design.md#components) | [Task 1](#task-1) | [AC-005-1](./verification.md#ac-005-1-req-005) | PASS — fixed solar & weekend checks verified |
| REQ-005 | AC-005-2 | [ADR-002](./decisions.md#adr-002-dynamic-holiday-and-compensatory-calculation-with-process-wide-cache) | [Interfaces](./design.md#interfaces-and-flow) | [Task 1](#task-1) | [AC-005-2](./verification.md#ac-005-2-req-005) | PASS — weekday non-holiday verified |
| REQ-006 | AC-006-1 | [ADR-002](./decisions.md#adr-002-dynamic-holiday-and-compensatory-calculation-with-process-wide-cache) | [Interfaces](./design.md#interfaces-and-flow) | [Task 1](#task-1) | [AC-006-1](./verification.md#ac-006-1-req-006) | PASS — 2024 Tet Eve & Day 1 verified |
| REQ-006 | AC-006-2 | [ADR-002](./decisions.md#adr-002-dynamic-holiday-and-compensatory-calculation-with-process-wide-cache) | [Boundaries](./design.md#boundaries-and-recovery) | [Task 1](#task-1) | [AC-006-2](./verification.md#ac-006-2-req-006) | PASS — year-boundary & `tet1 - 1d` inspected |
| REQ-007 | AC-007-1 | [ADR-002](./decisions.md#adr-002-dynamic-holiday-and-compensatory-calculation-with-process-wide-cache) | [Boundaries](./design.md#boundaries-and-recovery) | [Task 1](#task-1) | [AC-007-1](./verification.md#ac-007-1-req-007) | PASS — 2011 compensatory chain verified |
| REQ-007 | AC-007-2 | [ADR-002](./decisions.md#adr-002-dynamic-holiday-and-compensatory-calculation-with-process-wide-cache) | [Interfaces](./design.md#interfaces-and-flow) | [Task 1](#task-1) | [AC-007-2](./verification.md#ac-007-2-req-007) | PASS — `HOLIDAY_CACHE` inspected & exercised |

- Backward scope check: PASS — only spec files created for this package; zero `.rs` modifications.
- Reference and catalog consistency: PASS — `REQ-005..REQ-007` and `ADR-002` align across all 5 package files and `docs/specs/INDEX.md`.
