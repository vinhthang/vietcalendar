# Tasks: Dynamic AI Agent Discovery & Synchronized Docs

Sources: [spec.md](./spec.md), [design.md](./design.md)

## Task 1

**Add dynamic `initialize.instructions` to `McpCorePlugin` (TDD)**

- [x] Complete the work and its applicable checks.
- Covers: REQ-043 / AC-043-1, AC-043-2
- Files: `src/plugins/mcp_core.rs`
- Dependencies: None
- Blocked by: None
- Work: Write failing unit test asserting live dynamic `result.instructions` updates on tool/template registration and disposal while staying valid when empty; implement `build_initialize_instructions` and update `McpCoreEngine::dispatch`.
- Completion evidence: `mcp_core` unit tests (`3 passed; 0 failed`) verified under `-D warnings` in [Run 1](./verification.md#run-1--2026-11-22).

## Task 2

**Implement `HttpDiscoveryPlugin` (`/llms.txt`, `/llms-full.txt`, `/.well-known/mcp.json`) (TDD)**

- [x] Complete the work and its applicable checks.
- Covers: REQ-042, REQ-043 / AC-042-1, AC-042-2, AC-043-2
- Files: `src/plugins/http_discovery.rs`, `src/plugins/mod.rs`
- Dependencies: Task 1
- Blocked by: None
- Work: Scaffold `http_discovery`, write failing tests for `sanitize_origin` header injection defense and live REST/MCP rendering across mount, `mcp_tools` dispose, and `mcp_core` dispose (`http-only` fallback); implement `HttpDiscoveryPlugin` with `OpenApi` fragment and verify GREEN.
- Completion evidence: `http_discovery` tests (`2 passed; 0 failed`) and `cargo mutants` (`21/25` viable = `84.0%` caught) verified in [Run 1](./verification.md#run-1--2026-11-22).

## Task 3

**Wire profile factory, E2E integration tests, and synchronized documentation**

- [x] Complete the work and its applicable checks.
- Covers: REQ-042, REQ-043 / AC-042-1, AC-042-2, AC-043-1
- Files: `src/profiles/mod.rs`, `config/cordis.yml`, `tests/integration_test.rs`, `llms.txt`, `README.md`, `docs/mcp_setup.md`, `AGENTS.md`
- Dependencies: Task 2
- Blocked by: None
- Work: Register `http_discovery` in `build_default_factories` and `config/cordis.yml` (`serve` & `http-only`), add E2E `serve` / `http-only` / `astrology_vn` dispose tests, author repo-root `llms.txt`, refresh `README.md` + `docs/mcp_setup.md` + `AGENTS.md`, and run `cargo test --offline --all-targets`, `cargo mutants`, `arch-guard`, and `code-review`.
- Completion evidence: Full `70` lib + `8` integration tests, `arch-guard` CLEAN, static zero-panic audit, and synchronized documentation verified in [Run 1](./verification.md#run-1--2026-11-22).

## Gate 2: traceability and readiness

| Requirement | AC | ADR or rationale | Design | Task | Planned verification | Status / reason |
| --- | --- | --- | --- | --- | --- | --- |
| REQ-042 | AC-042-1 | [ADR-013](./decisions.md#adr-013-wait-free-snapshot-agent-discovery-http_discovery--dynamic-mcp-initializeinstructions) | [Interfaces](./design.md#interfaces-and-flow) | [Task 2](#task-2), [Task 3](#task-3) | [AC-042-1](./verification.md#ac-042-1-req-042) | PASS — dynamic discovery & unmount mapped |
| REQ-042 | AC-042-2 | [ADR-013](./decisions.md#adr-013-wait-free-snapshot-agent-discovery-http_discovery--dynamic-mcp-initializeinstructions) | [Boundaries](./design.md#boundaries-and-recovery) | [Task 2](#task-2), [Task 3](#task-3) | [AC-042-2](./verification.md#ac-042-2-req-042) | PASS — `http-only` & `sanitize_origin` mapped |
| REQ-043 | AC-043-1 | [ADR-013](./decisions.md#adr-013-wait-free-snapshot-agent-discovery-http_discovery--dynamic-mcp-initializeinstructions) | [Components](./design.md#components) | [Task 1](#task-1), [Task 3](#task-3) | [AC-043-1](./verification.md#ac-043-1-req-043) | PASS — MCP instructions, `llms.txt` & docs mapped |
| REQ-043 | AC-043-2 | [Rationale](./decisions.md#architectural-rationale) | [Boundaries](./design.md#boundaries-and-recovery) | [Task 1](#task-1), [Task 2](#task-2) | [AC-043-2](./verification.md#ac-043-2-req-043) | PASS — empty MCP & OpenAPI safety mapped |

- Backward scope check: PASS — Tasks 1–3 add discovery & documentation cleanly with zero breakage to frozen `REQ-001..REQ-041`.
- Reference and catalog consistency: PASS — `REQ-042..REQ-043`, `ADR-013`, and `docs/specs/INDEX.md` are aligned.
