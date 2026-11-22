# Tasks: Vietnamese Luni-Solar Astronomical Calendar Engine

Sources: [spec.md](./spec.md), [design.md](./design.md)

## Task 1

**Verify astronomical calendar baseline and tests**

- [x] Complete the work and its applicable checks.
- Covers: REQ-001, REQ-002, REQ-003, REQ-004 / AC-001-1, AC-001-2, AC-002-1, AC-002-2, AC-003-1, AC-003-2, AC-004-1, AC-004-2
- Files: `docs/specs/2026-11-21-astronomical-calendar/spec.md`, `docs/specs/2026-11-21-astronomical-calendar/decisions.md`, `docs/specs/2026-11-21-astronomical-calendar/design.md`, `docs/specs/2026-11-21-astronomical-calendar/tasks.md`, `docs/specs/2026-11-21-astronomical-calendar/verification.md`
- Dependencies: None
- Blocked by: None
- Work: Audit `src/calendar.rs`, `src/models.rs`, `src/services.rs`, and `tests/integration_test.rs`; run `cargo test --offline`; document baseline contracts and `TD-03`/`TD-04` tech debt without modifying `.rs` files.
- Completion evidence: `cargo test --offline` passed (all 4 `calendar`, 2 `models`, 3 calendar `services`, and 5 calendar `integration_test` cases green) plus source verification recorded in [Run 1](./verification.md#run-1--2026-11-21).

## Gate 2: traceability and readiness

| Requirement | AC | ADR or rationale | Design | Task | Planned verification | Status / reason |
| --- | --- | --- | --- | --- | --- | --- |
| REQ-001 | AC-001-1 | [ADR-001](./decisions.md#adr-001-dedicated-lunardate-domain-model-and-pure-rust-astronomical-computation) | [Components](./design.md#components) | [Task 1](#task-1) | [AC-001-1](./verification.md#ac-001-1-req-001) | PASS — JD roundtrip verified |
| REQ-001 | AC-001-2 | [ADR-001](./decisions.md#adr-001-dedicated-lunardate-domain-model-and-pure-rust-astronomical-computation) | [Boundaries](./design.md#boundaries-and-recovery) | [Task 1](#task-1) | [AC-001-2](./verification.md#ac-001-2-req-001) | PASS — 1582 boundary verified |
| REQ-002 | AC-002-1 | [ADR-001](./decisions.md#adr-001-dedicated-lunardate-domain-model-and-pure-rust-astronomical-computation) | [Interfaces](./design.md#interfaces-and-flow) | [Task 1](#task-1) | [AC-002-1](./verification.md#ac-002-1-req-002) | PASS — AA98 bounds verified |
| REQ-002 | AC-002-2 | [ADR-001](./decisions.md#adr-001-dedicated-lunardate-domain-model-and-pure-rust-astronomical-computation) | [Interfaces](./design.md#interfaces-and-flow) | [Task 1](#task-1) | [AC-002-2](./verification.md#ac-002-2-req-002) | PASS — `t < -11.0` inspected |
| REQ-003 | AC-003-1 | [ADR-001](./decisions.md#adr-001-dedicated-lunardate-domain-model-and-pure-rust-astronomical-computation) | [Interfaces](./design.md#interfaces-and-flow) | [Task 1](#task-1) | [AC-003-1](./verification.md#ac-003-1-req-003) | PASS — solar/lunar conversions verified |
| REQ-003 | AC-003-2 | [ADR-001](./decisions.md#adr-001-dedicated-lunardate-domain-model-and-pure-rust-astronomical-computation) | [Boundaries](./design.md#boundaries-and-recovery) | [Task 1](#task-1) | [AC-003-2](./verification.md#ac-003-2-req-003) | PASS — invalid leap rejection verified |
| REQ-004 | AC-004-1 | [ADR-001](./decisions.md#adr-001-dedicated-lunardate-domain-model-and-pure-rust-astronomical-computation) | [Interfaces](./design.md#interfaces-and-flow) | [Task 1](#task-1) | [AC-004-1](./verification.md#ac-004-1-req-004) | PASS — Metonic & Serde verified |
| REQ-004 | AC-004-2 | [ADR-001](./decisions.md#adr-001-dedicated-lunardate-domain-model-and-pure-rust-astronomical-computation) | [Boundaries](./design.md#boundaries-and-recovery) | [Task 1](#task-1) | [AC-004-2](./verification.md#ac-004-2-req-004) | PASS — `LunarDate` Serde & `TD-04` verified |

- Backward scope check: PASS — only spec files created for this package; zero `.rs` modifications.
- Reference and catalog consistency: PASS — `REQ-001..REQ-004` and `ADR-001` align across `spec.md`, `decisions.md`, `design.md`, `verification.md`, and `docs/specs/INDEX.md`.
