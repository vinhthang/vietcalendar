# Tasks: HTTP REST API and OpenAPI Specification

Sources: [spec.md](./spec.md), [design.md](./design.md)

## Task 1

**Verify HTTP REST and OpenAPI baseline and tests**

- [x] Complete the work and its applicable checks.
- Covers: REQ-008, REQ-009, REQ-010, REQ-011, REQ-012 / AC-008-1, AC-008-2, AC-009-1, AC-009-2, AC-010-1, AC-010-2, AC-011-1, AC-011-2, AC-012-1, AC-012-2
- Files: `docs/specs/2026-11-21-http-rest-openapi/spec.md`, `docs/specs/2026-11-21-http-rest-openapi/decisions.md`, `docs/specs/2026-11-21-http-rest-openapi/design.md`, `docs/specs/2026-11-21-http-rest-openapi/tasks.md`, `docs/specs/2026-11-21-http-rest-openapi/verification.md`
- Dependencies: None
- Blocked by: None
- Work: Audit `src/handlers.rs` and `src/main.rs`; run `cargo test --offline` across all 6 handler unit tests; document baseline endpoints and `TD-06` OpenAPI/extractor quirks without modifying `.rs` code.
- Completion evidence: `cargo test --offline` passed all 6 `handlers::tests::*` tests plus source verification recorded in [Run 1](./verification.md#run-1--2026-11-21).

## Gate 2: traceability and readiness

| Requirement | AC | ADR or rationale | Design | Task | Planned verification | Status / reason |
| --- | --- | --- | --- | --- | --- | --- |
| REQ-008 | AC-008-1 | [Rationale](./decisions.md#architectural-rationale) | [Components](./design.md#components) | [Task 1](#task-1) | [AC-008-1](./verification.md#ac-008-1-req-008) | PASS — `home()` UTC+7 code inspected |
| REQ-008 | AC-008-2 | [Rationale](./decisions.md#architectural-rationale) | [Interfaces](./design.md#interfaces-and-flow) | [Task 1](#task-1) | [AC-008-2](./verification.md#ac-008-2-req-008) | PASS — zero-input `home()` signature inspected |
| REQ-009 | AC-009-1 | [ADR-003](./decisions.md#adr-003-symmetric-iso-8601-convertdate-routes-alongside-legacy-lunar-query-route) | [Interfaces](./design.md#interfaces-and-flow) | [Task 1](#task-1) | [AC-009-1](./verification.md#ac-009-1-req-009) | PASS — `test_get_lunar_handler` verified |
| REQ-009 | AC-009-2 | [ADR-004](./decisions.md#adr-004-centralized-apperror-json-mapping-over-handler-string-errors) | [Boundaries](./design.md#boundaries-and-recovery) | [Task 1](#task-1) | [AC-009-2](./verification.md#ac-009-2-req-009) | PASS — invalid `/lunar` handler test verified |
| REQ-010 | AC-010-1 | [ADR-003](./decisions.md#adr-003-symmetric-iso-8601-convertdate-routes-alongside-legacy-lunar-query-route) | [Interfaces](./design.md#interfaces-and-flow) | [Task 1](#task-1) | [AC-010-1](./verification.md#ac-010-1-req-010) | PASS — `/convert/*` handler tests verified |
| REQ-010 | AC-010-2 | [ADR-004](./decisions.md#adr-004-centralized-apperror-json-mapping-over-handler-string-errors) | [Boundaries](./design.md#boundaries-and-recovery) | [Task 1](#task-1) | [AC-010-2](./verification.md#ac-010-2-req-010) | PASS — invalid format & leap rejection verified |
| REQ-011 | AC-011-1 | [Rationale](./decisions.md#architectural-rationale) | [Interfaces](./design.md#interfaces-and-flow) | [Task 1](#task-1) | [AC-011-1](./verification.md#ac-011-1-req-011) | PASS — `test_check_vietnam_holiday_handler` verified |
| REQ-011 | AC-011-2 | [ADR-004](./decisions.md#adr-004-centralized-apperror-json-mapping-over-handler-string-errors) | [Boundaries](./design.md#boundaries-and-recovery) | [Task 1](#task-1) | [AC-011-2](./verification.md#ac-011-2-req-011) | PASS — `NaiveDate::from_ymd_opt` guard inspected |
| REQ-012 | AC-012-1 | [ADR-004](./decisions.md#adr-004-centralized-apperror-json-mapping-over-handler-string-errors) | [Boundaries](./design.md#boundaries-and-recovery) | [Task 1](#task-1) | [AC-012-1](./verification.md#ac-012-1-req-012) | PASS — `AppError` & `ApiDoc` verified |
| REQ-012 | AC-012-2 | [ADR-004](./decisions.md#adr-004-centralized-apperror-json-mapping-over-handler-string-errors) | [Boundaries](./design.md#boundaries-and-recovery) | [Task 1](#task-1) | [AC-012-2](./verification.md#ac-012-2-req-012) | PASS — zero `Result<Json<T>, String>` inspected |

- Backward scope check: PASS — only spec files created for this package; zero `.rs` modifications.
- Reference and catalog consistency: PASS — `REQ-008..REQ-012` and `ADR-003, ADR-004` align across all 5 package files and `docs/specs/INDEX.md`.
