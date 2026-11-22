# Tasks: Unknown MCP Tool Returns Invalid Params (`-32602`)

Sources: [spec.md](./spec.md), [design.md](./design.md)

## Task 1

**Return JSON-RPC `-32602` on unknown MCP tool name (`tools/call`)**

- [x] Complete the work and its applicable checks.
- Covers: REQ-044 / AC-044-1, AC-044-2
- Files: `src/plugins/mcp_core.rs`, `src/contracts/mcp.rs`
- Dependencies: None
- Blocked by: None
- Work: Update `test_mcp_core_protocol_dispatch_and_errors` to assert `-32602` for unknown tool names while preserving `-32601` for unknown JSON-RPC methods (RED), change `tools/call` miss error code in `McpCoreEngine::dispatch` from `-32601` to `-32602`, synchronize `McpError` rustdocs, and verify GREEN across the full test suite.
- Completion evidence: `plugins::mcp_core` (`3 passed; 0 failed`) and full CI parity suite (`72` unit + `8` integration tests, `0` warnings) verified in [Run 1](./verification.md#run-1--2026-11-22).

## Gate 2: traceability and readiness

| Requirement | AC | ADR or rationale | Design | Task | Planned verification | Status / reason |
| --- | --- | --- | --- | --- | --- | --- |
| REQ-044 | AC-044-1 | [Rationale](./decisions.md#architectural-rationale) | [Interfaces](./design.md#interfaces-and-flow) | [Task 1](#task-1) | [AC-044-1](./verification.md#ac-044-1-req-044) | PASS — valid tools & `-32601` method preservation mapped |
| REQ-044 | AC-044-2 | [Rationale](./decisions.md#architectural-rationale) | [Boundaries](./design.md#boundaries-and-recovery) | [Task 1](#task-1) | [AC-044-2](./verification.md#ac-044-2-req-044) | PASS — unknown tool `-32602` assertion mapped |

- Backward scope check: PASS — Task 1 touches only `src/plugins/mcp_core.rs` and `src/contracts/mcp.rs`.
- Reference and catalog consistency: PASS — `REQ-044` and `docs/specs/INDEX.md` are aligned.
