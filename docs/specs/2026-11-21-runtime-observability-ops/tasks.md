# Tasks: Runtime CLI, Observability, Containerization, and CI/CD

Sources: [spec.md](./spec.md), [design.md](./design.md)

## Task 1

**Verify runtime observability container and CI baseline**

- [x] Complete the work and its applicable checks.
- Covers: REQ-018, REQ-019, REQ-020, REQ-021 / AC-018-1, AC-018-2, AC-019-1, AC-019-2, AC-020-1, AC-020-2, AC-021-1, AC-021-2
- Files: `docs/specs/2026-11-21-runtime-observability-ops/spec.md`, `docs/specs/2026-11-21-runtime-observability-ops/decisions.md`, `docs/specs/2026-11-21-runtime-observability-ops/design.md`, `docs/specs/2026-11-21-runtime-observability-ops/tasks.md`, `docs/specs/2026-11-21-runtime-observability-ops/verification.md`
- Dependencies: None
- Blocked by: None
- Work: Audit `src/main.rs`, `src/bin/vietcalendar-mcp.rs`, `Cargo.toml`, `Dockerfile`, `.github/workflows/rust.yml`, `load_test.js`, and `conf/configuration.json`; verify `cargo test --offline` builds both binaries cleanly; document baseline contracts and `TD-07`/`TD-08` debt without modifying code.
- Completion evidence: `cargo test --offline` compiled `vietcalendar` and `vietcalendar-mcp` cleanly (exit code `0`) plus static audit recorded in [Run 1](./verification.md#run-1--2026-11-21).

## Gate 2: traceability and readiness

| Requirement | AC | ADR or rationale | Design | Task | Planned verification | Status / reason |
| --- | --- | --- | --- | --- | --- | --- |
| REQ-018 | AC-018-1 | [ADR-006](./decisions.md#adr-006-single-dual-mode-binary-with-two-stage-non-root-container-packaging) | [Interfaces](./design.md#interfaces-and-flow) | [Task 1](#task-1) | [AC-018-1](./verification.md#ac-018-1-req-018) | PASS — binary build & CLI inspected |
| REQ-018 | AC-018-2 | [ADR-006](./decisions.md#adr-006-single-dual-mode-binary-with-two-stage-non-root-container-packaging) | [Boundaries](./design.md#boundaries-and-recovery) | [Task 1](#task-1) | [AC-018-2](./verification.md#ac-018-2-req-018) | PASS — MCP stderr & unused config inspected |
| REQ-019 | AC-019-1 | [Rationale](./decisions.md#architectural-rationale) | [Interfaces](./design.md#interfaces-and-flow) | [Task 1](#task-1) | [AC-019-1](./verification.md#ac-019-1-req-019) | PASS — `init_telemetry` & `TraceLayer` inspected |
| REQ-019 | AC-019-2 | [Rationale](./decisions.md#architectural-rationale) | [Boundaries](./design.md#boundaries-and-recovery) | [Task 1](#task-1) | [AC-019-2](./verification.md#ac-019-2-req-019) | PASS — OTLP exporter fallback inspected |
| REQ-020 | AC-020-1 | [ADR-006](./decisions.md#adr-006-single-dual-mode-binary-with-two-stage-non-root-container-packaging) | [Components](./design.md#components) | [Task 1](#task-1) | [AC-020-1](./verification.md#ac-020-1-req-020) | PASS — `Dockerfile` stages & `appuser` inspected |
| REQ-020 | AC-020-2 | [ADR-006](./decisions.md#adr-006-single-dual-mode-binary-with-two-stage-non-root-container-packaging) | [Interfaces](./design.md#interfaces-and-flow) | [Task 1](#task-1) | [AC-020-2](./verification.md#ac-020-2-req-020) | PASS — `touch` invalidation inspected |
| REQ-021 | AC-021-1 | [Rationale](./decisions.md#architectural-rationale) | [Interfaces](./design.md#interfaces-and-flow) | [Task 1](#task-1) | [AC-021-1](./verification.md#ac-021-1-req-021) | PASS — `.github/workflows/rust.yml` inspected |
| REQ-021 | AC-021-2 | [Rationale](./decisions.md#architectural-rationale) | [Components](./design.md#components) | [Task 1](#task-1) | [AC-021-2](./verification.md#ac-021-2-req-021) | PASS — PR gate & `load_test.js` inspected |

- Backward scope check: PASS — only spec files created for this package; zero code/ops file modifications.
- Reference and catalog consistency: PASS — `REQ-018..REQ-021` and `ADR-006` align across all 5 package files and `docs/specs/INDEX.md`.
