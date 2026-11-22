# Unknown MCP Tool Returns Invalid Params (`-32602`)

- Feature ID: 2026-11-22-mcp-unknown-tool-invalid-params
- Type: bugfix
- Status: In-Testing
- First entered In-Testing: 2026-11-22
- Related specifications: Implemented: Changes [nonblocking-plugin-refactor/REQ-031](../2026-11-21-nonblocking-plugin-refactor/spec.md#req-031-pluggable-mcp-core-registry-and-unified-toolresource-plugin)

## Problem and source intent

Live audit finding (`P2`) on `https://vietcalendar.vinhthang.dev`: calling `tools/call` with an unknown tool name returned JSON-RPC `-32601` (`Method not found`) instead of `-32602` (`Invalid params`), followed by explicit user instruction: `"fix these issues"`. Under MCP `2024-11-05`, `tools/call` is a valid method and an unrecognized `params.name` is an invalid parameter (`-32602`), matching unknown resource URIs on `resources/read`.

## Repository grounding

- Dispatcher & error mapping: [`src/plugins/mcp_core.rs`](../../../src/plugins/mcp_core.rs) (`McpCoreEngine::dispatch`), [`src/contracts/mcp.rs`](../../../src/contracts/mcp.rs) (`McpError`).
- Unit tests: [`src/plugins/mcp_core.rs`](../../../src/plugins/mcp_core.rs) (`test_mcp_core_protocol_dispatch_and_errors`).

## Non-goals

- Changing `-32601` (`Method not found`) for unknown top-level JSON-RPC `method` names (`foo/bar`), or altering HTTP route `404` / MCP tool schemas.

## Requirements

### REQ-044: Unknown MCP Tool Returns Invalid Params

- Changes: [nonblocking-plugin-refactor/REQ-031](../2026-11-21-nonblocking-plugin-refactor/spec.md#req-031-pluggable-mcp-core-registry-and-unified-toolresource-plugin)
- Source intent: Live MCP protocol review finding (`P2`) and user instruction `"fix these issues"`.
- Previous contract: Calling `tools/call` with an unregistered tool `name` returned JSON-RPC error `-32601` (`Method not found`, matching legacy `REQ-014` `AC-014-2`).
- Statement: WHEN receiving a JSON-RPC `tools/call` request whose `params.name` matches no registered tool in the active `McpRegistryService` snapshot, THE `McpCorePlugin` dispatcher SHALL return JSON-RPC error code `-32602` (`"Tool not found: <name>"`), WHILE continuing to return `-32601` for unknown top-level JSON-RPC `method` values.
- AC-044-1 — Success: `tools/call` with valid registered tool names succeeds, and an unregistered top-level JSON-RPC method (e.g., `"does/not/exist"`) still returns `-32601`.
- AC-044-2 — Boundary/failure: IF `tools/call` supplies an unknown `params.name` (e.g., `"missing"`), THEN `McpCoreEngine::dispatch` SHALL return JSON-RPC error code `-32602` with message `"Tool not found: missing"`.

## Assumptions and owner-deferred questions

- No `[PROVISIONAL]` items remain open; existing tool schemas and HTTP transports are unaffected.

## Approvals and lifecycle history

- 2026-11-22 (`Planning`): Delta package created for live MCP review fix (`P2`) authorized by `"fix these issues"`.
- 2026-11-22 (`In-Progress -> In-Testing`): [Task 1](./tasks.md#task-1) and [Run 1](./verification.md#run-1--2026-11-22) passed; awaiting live deployment confirmation before promoting to `Verified`.
