# Model Context Protocol (MCP) Server and Dual Transports

- Feature ID: 2026-11-21-mcp-server
- Type: enhancement
- Status: In-Testing
- First entered In-Testing: 2026-11-21
- Related specifications: None

## Problem and source intent

Brownfield specification baseline requested by user: "Analyze this project, write AGENTS.md, write specs for existing features using my `spec-driven` skill... document current behavior and catalog discovered quirks/tech debt". Captures the JSON-RPC 2.0 Model Context Protocol (`2024-11-05`) engine, 5 tools, 2 resource URIs, `stdio` transport, HTTP/SSE session streaming, and direct HTTP POST endpoints.

## Repository grounding

- Legacy `src/mcp.rs` (migrated to `src/plugins/mcp_{core,tools,stdio,http}.rs`): `JsonRpcRequest`, `JsonRpcResponse`, `JsonRpcError`, `handle_request`, `parse_solar_input`, `HolidayInfo`, `compute_holiday_list_for_year`, `run_stdio_server`, `McpSessionManager`, `SessionStream`, `sse_handler`, `MessageQuery`, `message_handler`, `direct_post_handler`.
- [`src/main.rs`](../../../src/main.rs): `serve` and `mcp` CLI subcommands.
- [`src/bin/vietcalendar-mcp.rs`](../../../src/bin/vietcalendar-mcp.rs): Standalone `stdio` binary wrapper.
- [`docs/ARCHITECTURE_DECISIONS.md`](../../ARCHITECTURE_DECISIONS.md): ADR-8 (MCP Server Support) and ADR-11 (Public Remote MCP over HTTP/SSE with Permissive CORS).

## Non-goals

- Fixing `compute_holiday_list_for_year` missing compensatory leave (`TD-01`), missing `convert_lunar_to_solar` day/month range checks (`TD-03`), or unbounded SSE channel / notification `sessionId` validation (`TD-07`) without a linked delta specification.
- Documenting CLI flag parsing or OpenTelemetry setup (covered in [runtime-observability-ops](../2026-11-21-runtime-observability-ops/spec.md)).

## Requirements

### REQ-013: JSON-RPC 2.0 Lifecycle, Ping, and Error Dispatch

- Source intent: Baseline behavior in `src/mcp.rs::handle_request` and `run_stdio_server` (`ADR-8`, verified by `test_mcp_initialize`).
- Statement: WHEN receiving JSON-RPC 2.0 requests (`initialize`, `ping`, `notifications/initialized`, or malformed/unknown methods), THE MCP engine SHALL advertise `protocolVersion: "2024-11-05"` and `serverInfo` (`vietcalendar-mcp`, `0.1.0-alpha (experimental)`) with `tools` and `resources` capabilities, suppress responses for `notifications/initialized`, and emit JSON-RPC errors `-32700`, `-32601`, or `-32602` on parse, method, or parameter failures.
- AC-013-1 — Success: `initialize` returns `protocolVersion == "2024-11-05"`, `tools` and `resources` capabilities, and `serverInfo.name == "vietcalendar-mcp"`; `ping` returns empty object `{}`; `notifications/initialized` returns `None`.
- AC-013-2 — Boundary/failure: Unknown methods return JSON-RPC error `-32601`; missing `params` or `name` returns `-32602`; invalid JSON on `stdio` emits `-32700`.

### REQ-014: Five-Tool MCP Catalog (`tools/list` and `tools/call`)

- Current contract: Changed by [nonblocking-plugin-refactor/REQ-031](../2026-11-21-nonblocking-plugin-refactor/spec.md#req-031-pluggable-mcp-core-registry-and-unified-toolresource-plugin)
- Source intent: Baseline behavior in `src/mcp.rs::handle_request` (`tools/list`, `tools/call`, `parse_solar_input`, `compute_holiday_list_for_year`) verified by `test_mcp_tools_list`, `test_mcp_convert_solar_to_lunar_call`, and `test_mcp_convert_lunar_to_solar_call`.
- Statement: WHEN handling `tools/list` or `tools/call`, THE MCP engine SHALL expose and execute 5 tools (`get_today_lunar`, `convert_solar_to_lunar`, `convert_lunar_to_solar`, `check_vietnam_holiday`, `get_year_holidays`), returning pretty-printed JSON inside `content[0].text` (`type: "text"`).
- AC-014-1 — Success: `tools/list` returns exactly 5 tool definitions; `convert_solar_to_lunar` (`"2024-02-10"`) yields lunar `1/1/2024`; `convert_lunar_to_solar` (`1/1/2024`, `is_leap_month: false`) yields `"2024-02-10"`; `get_year_holidays` returns `{ year, total_holidays, holidays }`.
- AC-014-2 — Boundary/failure: IF an unknown tool name is called, THEN `handle_request` returns `-32601`; IF required parameters fail validation or `to_solar` rejects an invalid leap month, THEN `handle_request` returns `-32602` (noting documented `TD-01` missing compensatory leave in `compute_holiday_list_for_year` and `TD-03` unchecked `day`/`month` in `tools/call`).

### REQ-015: MCP Calendar Resources (`resources/list` and `resources/read`)

- Current contract: Changed by [nonblocking-plugin-refactor/REQ-031](../2026-11-21-nonblocking-plugin-refactor/spec.md#req-031-pluggable-mcp-core-registry-and-unified-toolresource-plugin)
- Source intent: Baseline behavior in `src/mcp.rs::handle_request` verified by `test_mcp_resources`.
- Statement: WHEN handling `resources/list` or `resources/read`, THE MCP engine SHALL advertise `calendar://today` and `calendar://holidays/2026`, and serve live JSON snapshots for `calendar://today` and any valid integer `calendar://holidays/{year}` URI.
- AC-015-1 — Success: `resources/list` returns at least 2 `application/json` resources; `resources/read` for `calendar://today` returns UTC+7 solar/lunar/holiday state and `calendar://holidays/2024` returns non-empty serialized `HolidayInfo` array JSON.
- AC-015-2 — Boundary/failure: IF `resources/read` receives missing `params`, missing `uri`, a non-integer year under `calendar://holidays/`, or an unknown URI, THEN `handle_request` SHALL return JSON-RPC error `-32602` (noting `TD-01` missing compensatory leave in `compute_holiday_list_for_year`).

### REQ-016: Local Stdio JSON-RPC Transport

- Source intent: Baseline behavior in `src/mcp.rs::run_stdio_server`, `src/main.rs::Commands::Mcp`, and `src/bin/vietcalendar-mcp.rs` (`ADR-8`).
- Statement: WHEN invoked via `vietcalendar mcp` or `vietcalendar-mcp`, THE MCP server SHALL read newline-delimited JSON-RPC 2.0 messages from `stdin`, ignore blank lines, and write serialized single-line JSON responses (flushing after each message) to `stdout`.
- AC-016-1 — Success: Valid single-line JSON-RPC requests yield one newline-delimited JSON response on `stdout`, while notifications (`None` return from `handle_request`) produce no `stdout` line.
- AC-016-2 — Boundary/failure: Blank lines are skipped without error output, and malformed JSON lines emit a JSON-RPC `-32700` `"Parse error: Invalid JSON"` line rather than terminating the loop.

### REQ-017: Remote HTTP/SSE and Direct HTTP JSON-RPC Transports

- Current contract: Changed by [nonblocking-plugin-refactor/REQ-032](../2026-11-21-nonblocking-plugin-refactor/spec.md#req-032-non-blocking-bounded-mcp-httpsse-session-plugin)
- Source intent: Baseline behavior in `src/mcp.rs::McpSessionManager`, `SessionStream`, `sse_handler`, `message_handler`, `direct_post_handler`, and `src/main.rs` (`ADR-11`, verified by `test_mcp_session_manager`).
- Statement: WHEN clients connect over remote HTTP MCP routes, THE Axum server SHALL register UUID SSE sessions in `McpSessionManager`, push `event: endpoint` (`/mcp/message?sessionId=...`) on `GET /mcp/sse`, route `POST /mcp/message` responses as `event: message` (`202 Accepted` or `404 Not Found`), automatically unregister dropped `SessionStream` connections, and respond synchronously to direct JSON-RPC calls on `POST /mcp` and `POST /mcp/sse` (`200 OK` or `204 No Content`).
- AC-017-1 — Success: `McpSessionManager` registers sessions, increments `active_sessions_count()`, delivers `Event` frames via `send_to_session`, and cleans up on `remove()` or `SessionStream::drop`.
- AC-017-2 — Boundary/failure: IF `POST /mcp/message` references an unknown or closed `sessionId` for a request producing a response, THEN `message_handler` returns `HTTP 404 Not Found`; IF `direct_post_handler` receives a notification (`None`), THEN it returns `HTTP 204 No Content` (noting `TD-07` notification `sessionId` skip on `POST /mcp/message`).

## Assumptions and owner-deferred questions

- **Documented baseline quirks (`TD-01`, `TD-03`, `TD-07`):** `compute_holiday_list_for_year` omits weekend compensatory leave (*nghỉ bù*) and deduplicates same-day events by date only; MCP `convert_lunar_to_solar` skips `1..=12`/`1..=30` validation and uses `as i32`; `McpSessionManager` uses `mpsc::unbounded_channel` and skips `sessionId` lookup on notifications; `resources/list` hardcodes `2026`. All remain documented in [`docs/PROJECT_ANALYSIS.md`](../../PROJECT_ANALYSIS.md) until dedicated delta specs authorize fixes.

## Approvals and lifecycle history

- 2026-11-21 (`Planning -> In-Progress`): Authorized via `/goal` and settled plan answers ("Advance all 5 baseline specs to In-Testing with real execution evidence") covering `REQ-013..REQ-017` after Gate 1 and Gate 2 passed.
- 2026-11-21 (`In-Progress -> In-Testing`): [Task 1](./tasks.md#task-1) and [Run 1](./verification.md#run-1--2026-11-21) passed; awaiting human confirmation on real-world usage before promoting to `Verified`.
