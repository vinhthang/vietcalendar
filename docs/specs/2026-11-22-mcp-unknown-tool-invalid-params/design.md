# Design: Unknown MCP Tool Returns Invalid Params (`-32602`)

Sources: [spec.md](./spec.md), [decisions.md](./decisions.md)

## Components

| Requirement / AC | Path and symbol | Responsibility / change |
| --- | --- | --- |
| REQ-044 / AC-044-1, AC-044-2 | `src/plugins/mcp_core.rs` (`McpCoreEngine::dispatch`) | Return JSON-RPC `-32602` when `tools/call` specifies an unregistered tool name while retaining `-32601` for unknown methods. |
| REQ-044 / AC-044-2 | `src/contracts/mcp.rs` (`McpError`) | Synchronize `MethodNotFound` / `InvalidParams` rustdoc descriptions. |

## Interfaces and flow

1. Client calls `tools/call` with `{ "name": "missing" }`.
2. `McpCoreEngine::dispatch` loads the wait-free `ArcSwap` `RegistrySnapshot` and looks up `snap.tools.get(name)`.
3. When absent, it returns `JsonRpcResponse::error(id, -32602, format!("Tool not found: {name}"))`.
4. Unknown top-level JSON-RPC methods still match the catch-all arm returning `-32601`.

## Boundaries and recovery

- Zero locking or panics added; `McpError::code()` remains unchanged for handler errors (`-32602` for `InvalidParams`, `-32601` for `MethodNotFound`).

## Dependencies and unresolved design work

None.
