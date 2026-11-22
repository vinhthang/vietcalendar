# Tasks: Model Context Protocol (MCP) Server and Dual Transports

Sources: [spec.md](./spec.md), [design.md](./design.md)

## Task 1

**Verify MCP server baseline and tests**

- [x] Complete the work and its applicable checks.
- Covers: REQ-013, REQ-014, REQ-015, REQ-016, REQ-017 / AC-013-1, AC-013-2, AC-014-1, AC-014-2, AC-015-1, AC-015-2, AC-016-1, AC-016-2, AC-017-1, AC-017-2
- Files: `docs/specs/2026-11-21-mcp-server/spec.md`, `docs/specs/2026-11-21-mcp-server/decisions.md`, `docs/specs/2026-11-21-mcp-server/design.md`, `docs/specs/2026-11-21-mcp-server/tasks.md`, `docs/specs/2026-11-21-mcp-server/verification.md`
- Dependencies: None
- Blocked by: None
- Work: Audit `src/mcp.rs`, `src/main.rs`, and `src/bin/vietcalendar-mcp.rs`; run `cargo test --offline` across all 6 MCP unit tests; document baseline tools, resources, transports, and `TD-01`/`TD-03`/`TD-07` quirks without modifying `.rs` files.
- Completion evidence: `cargo test --offline` passed all 6 `mcp::tests::*` cases plus source verification recorded in [Run 1](./verification.md#run-1--2026-11-21).

## Gate 2: traceability and readiness

| Requirement | AC | ADR or rationale | Design | Task | Planned verification | Status / reason |
| --- | --- | --- | --- | --- | --- | --- |
| REQ-013 | AC-013-1 | [ADR-005](./decisions.md#adr-005-native-single-binary-dual-transport-json-rpc-20-mcp-engine) | [Interfaces](./design.md#interfaces-and-flow) | [Task 1](#task-1) | [AC-013-1](./verification.md#ac-013-1-req-013) | PASS — `test_mcp_initialize` verified |
| REQ-013 | AC-013-2 | [ADR-005](./decisions.md#adr-005-native-single-binary-dual-transport-json-rpc-20-mcp-engine) | [Boundaries](./design.md#boundaries-and-recovery) | [Task 1](#task-1) | [AC-013-2](./verification.md#ac-013-2-req-013) | PASS — JSON-RPC error branches inspected |
| REQ-014 | AC-014-1 | [ADR-005](./decisions.md#adr-005-native-single-binary-dual-transport-json-rpc-20-mcp-engine) | [Components](./design.md#components) | [Task 1](#task-1) | [AC-014-1](./verification.md#ac-014-1-req-014) | PASS — tools list & conversion calls verified |
| REQ-014 | AC-014-2 | [ADR-005](./decisions.md#adr-005-native-single-binary-dual-transport-json-rpc-20-mcp-engine) | [Boundaries](./design.md#boundaries-and-recovery) | [Task 1](#task-1) | [AC-014-2](./verification.md#ac-014-2-req-014) | PASS — tool errors & `TD-01`/`TD-03` inspected |
| REQ-015 | AC-015-1 | [ADR-005](./decisions.md#adr-005-native-single-binary-dual-transport-json-rpc-20-mcp-engine) | [Components](./design.md#components) | [Task 1](#task-1) | [AC-015-1](./verification.md#ac-015-1-req-015) | PASS — `test_mcp_resources` verified |
| REQ-015 | AC-015-2 | [ADR-005](./decisions.md#adr-005-native-single-binary-dual-transport-json-rpc-20-mcp-engine) | [Boundaries](./design.md#boundaries-and-recovery) | [Task 1](#task-1) | [AC-015-2](./verification.md#ac-015-2-req-015) | PASS — resource error branches inspected |
| REQ-016 | AC-016-1 | [ADR-005](./decisions.md#adr-005-native-single-binary-dual-transport-json-rpc-20-mcp-engine) | [Interfaces](./design.md#interfaces-and-flow) | [Task 1](#task-1) | [AC-016-1](./verification.md#ac-016-1-req-016) | PASS — `run_stdio_server` inspected |
| REQ-016 | AC-016-2 | [ADR-005](./decisions.md#adr-005-native-single-binary-dual-transport-json-rpc-20-mcp-engine) | [Boundaries](./design.md#boundaries-and-recovery) | [Task 1](#task-1) | [AC-016-2](./verification.md#ac-016-2-req-016) | PASS — blank-line & `-32700` handling inspected |
| REQ-017 | AC-017-1 | [ADR-005](./decisions.md#adr-005-native-single-binary-dual-transport-json-rpc-20-mcp-engine) | [Interfaces](./design.md#interfaces-and-flow) | [Task 1](#task-1) | [AC-017-1](./verification.md#ac-017-1-req-017) | PASS — `test_mcp_session_manager` verified |
| REQ-017 | AC-017-2 | [ADR-005](./decisions.md#adr-005-native-single-binary-dual-transport-json-rpc-20-mcp-engine) | [Boundaries](./design.md#boundaries-and-recovery) | [Task 1](#task-1) | [AC-017-2](./verification.md#ac-017-2-req-017) | PASS — HTTP/SSE handlers inspected |

- Backward scope check: PASS — only spec files created for this package; zero `.rs` modifications.
- Reference and catalog consistency: PASS — `REQ-013..REQ-017` and `ADR-005` align across all 5 package files and `docs/specs/INDEX.md`.
