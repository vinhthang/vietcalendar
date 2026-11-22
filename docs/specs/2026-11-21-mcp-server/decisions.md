# Decisions: Model Context Protocol (MCP) Server and Dual Transports

Governing specification: [spec.md](./spec.md)

## Architectural rationale

- Governs: REQ-013, REQ-014, REQ-015, REQ-016, REQ-017

The MCP layer (`src/mcp.rs`) shares the same `vietcalendar_rs` library crate as HTTP REST handlers, routing `get_today_lunar`, `convert_solar_to_lunar`, `convert_lunar_to_solar`, and `check_vietnam_holiday` directly through [astronomical-calendar](../2026-11-21-astronomical-calendar/spec.md#req-003-bidirectional-solar-and-vietnamese-lunar-conversion) and [vietnam-holidays](../2026-11-21-vietnam-holidays/spec.md#req-005-weekend-and-fixed-solar-holiday-classification). Only `get_year_holidays` and `calendar://holidays/{year}` currently invoke a module-local `compute_holiday_list_for_year` (`TD-01`). See `ADR-005` for the unified transport architecture.

## ADR-005: Native Single-Binary Dual-Transport JSON-RPC 2.0 MCP Engine

- Governs: REQ-013, REQ-014, REQ-015, REQ-016, REQ-017
- State: Accepted
- Context and constraints: AI developer clients require either local `stdio` process execution (Claude Desktop, Cursor, Antigravity) or remote HTTP access (legacy SSE `GET /mcp/sse` + `POST /mcp/message` and modern streamable direct `POST /mcp` / `POST /mcp/sse` with permissive CORS), without maintaining separate codebases or sidecar proxies.
- Viable options: Option A — Implement a lightweight custom JSON-RPC 2.0 dispatcher (`handle_request`) directly inside `src/mcp.rs` wired into both `run_stdio_server` (`vietcalendar mcp` / `vietcalendar-mcp`) and Axum routes (`McpSessionManager` SSE + `direct_post_handler`); vs. Option B — Run an external Node/Python MCP bridge sidecar in front of the HTTP REST endpoints.
- Decision: Option A — Native single-binary dual-transport MCP implementation inside `src/mcp.rs`.
- Costs and risks: Zero extra runtime processes or proxy latency, full binary parity across `stdio` and HTTP, and permissive CORS (`ADR-11`). Current implementation incurs known tech debt (`TD-01` duplicate holiday-list logic missing *nghỉ bù*, `TD-03` missing lunar range validation, `TD-07` unbounded SSE channels and notification `sessionId` omission).
- Rejected alternatives: Option B doubles container complexity, breaks minimal `<40MB`-class single-binary distribution (`Dockerfile`), and introduces IPC serialization overhead.
- Affected components: `src/mcp.rs`, `src/main.rs`, `src/bin/vietcalendar-mcp.rs`.

## Gate 1: normative review

| Check | Status | Evidence / required correction |
| --- | --- | --- |
| Codebase grounding | PASS | Grounded in `src/mcp.rs`, `src/main.rs`, `src/bin/vietcalendar-mcp.rs`, `README.md`, `docs/mcp_setup.md`, and `docs/ARCHITECTURE_DECISIONS.md` (ADR-8, ADR-11). |
| Attributable intent and explicit scope | PASS | `REQ-013`..`REQ-017` trace to brownfield audit and ADR-8/11; `TD-01`/`TD-03`/`TD-07` code remediations explicitly excluded in `Non-goals`. |
| Observable requirements and meaningful acceptance criteria | PASS | `AC-013-1`..`AC-017-2` specify protocol version `2024-11-05`, JSON-RPC codes (`-32700`, `-32601`, `-32602`), 5 tools, 2 resources, `stdio`, SSE `404`/`202`, and `204`. |
| Assumptions and owner-deferred questions are explicit | PASS | `TD-01`, `TD-03`, and `TD-07` baseline quirks explicitly documented under `Assumptions`; zero `[PROVISIONAL]` or `TODO` markers. |
| Architectural rationale respects requirements | PASS | `ADR-005` and `## Architectural rationale` govern `REQ-013`..`REQ-017` faithfully against existing transports. |
| IDs, references, and revision route are consistent | PASS | `REQ-013..REQ-017` and `ADR-005` match `docs/specs/INDEX.md`; cross-spec links valid. |
