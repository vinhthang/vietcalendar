# Decisions: Unknown MCP Tool Returns Invalid Params (`-32602`)

Governing specification: [spec.md](./spec.md)

## Architectural rationale

- Governs: REQ-044

In MCP JSON-RPC `2.0` (`2024-11-05`), `tools/call` itself is a valid method; passing an unknown `params.name` is an invalid parameter (`-32602`, consistent with `resources/read` returning `-32602` for unknown URIs in `mcp_core.rs`). Only unknown top-level `req.method` values map to `-32601` (`Method not found`). No multi-option architectural tradeoff exists, so changing the literal error code in `McpCoreEngine::dispatch` and synchronizing `McpError` rustdocs suffices without new ADR entries.

## Gate 1: normative review

| Check | Status | Evidence / required correction |
| --- | --- | --- |
| Codebase grounding | PASS | Grounded against inspected `src/plugins/mcp_core.rs` (`tools/call` branch) and `src/contracts/mcp.rs` (`McpError`). |
| Attributable intent and explicit scope | PASS | Traces directly to live review `P2` finding and `"fix these issues"`; top-level `-32601` explicitly preserved in `Non-goals`. |
| Observable requirements and meaningful acceptance criteria | PASS | `REQ-044` / `AC-044-1`–`AC-044-2` verify both `-32602` on unknown tool name and `-32601` preservation on unknown methods. |
| Assumptions and owner-deferred questions are explicit | PASS | Zero `[PROVISIONAL]` items; wire change bounded cleanly as Case 3 linked delta. |
| Architectural rationale respects requirements | PASS | Aligns `tools/call` error mapping with `resources/read` (`-32602`) while leaving registry snapshot dispatch untouched. |
| IDs, references, and revision route are consistent | PASS | Minted `REQ-044` after `REQ-043`; targets currently governing `nonblocking-plugin-refactor/REQ-031` cleanly. |
