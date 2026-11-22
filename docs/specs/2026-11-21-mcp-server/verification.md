# Verification: Model Context Protocol (MCP) Server and Dual Transports

Governing specification: [spec.md](./spec.md)

## Planned checks

### AC-013-1 (REQ-013)

- Method: Unit test `mcp::tests::test_mcp_initialize` and source inspection of `src/mcp.rs:72-90`.
- Expected outcome: `initialize` returns `"2024-11-05"`, `"vietcalendar-mcp"`, `"0.1.0-alpha (experimental)"`; `ping` returns `{}`; `notifications/initialized` returns `None`.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-013-2 (REQ-013)

- Method: Source inspection of `src/mcp.rs:169-176,404-409,556`.
- Expected outcome: Invalid JSON emits `-32700`, unknown method emits `-32601`, and missing `params` or `name` emits `-32602`.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-014-1 (REQ-014)

- Method: Unit tests `mcp::tests::test_mcp_tools_list`, `test_mcp_convert_solar_to_lunar_call`, `test_mcp_convert_lunar_to_solar_call`, plus inspection of `get_year_holidays` (`L286-L304`).
- Expected outcome: `tools/list` returns 5 tools; `"2024-02-10"` converts to lunar `1/1/2024` and back; `get_year_holidays` emits `{ year, total_holidays, holidays }`.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-014-2 (REQ-014)

- Method: Source inspection of `src/mcp.rs:197-200,213-273,305-311,448-534`.
- Expected outcome: Unknown tool returns `-32601`; missing/invalid args or `to_solar(...) == None` returns `-32602`; `TD-01`/`TD-03` baseline gaps are documented accurately.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-015-1 (REQ-015)

- Method: Unit test `mcp::tests::test_mcp_resources` and source inspection of `src/mcp.rs:324-388`.
- Expected outcome: `resources/list` returns 2 resources; `calendar://holidays/2024` resolves non-empty JSON; `calendar://today` resolves UTC+7 JSON.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-015-2 (REQ-015)

- Method: Source inspection of `src/mcp.rs:344-351,389-402`.
- Expected outcome: Missing `params`, missing `uri`, non-integer year, or unknown URI returns `-32602`.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-016-1 (REQ-016)

- Method: Source inspection of `src/mcp.rs::run_stdio_server` (`L537-L565`) and `src/bin/vietcalendar-mcp.rs`.
- Expected outcome: Reads `stdin` line by line, invokes `handle_request`, writes serialized JSON + `\n`, and flushes `stdout` when `Some(resp)`.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-016-2 (REQ-016)

- Method: Source inspection of `src/mcp.rs:543-561`.
- Expected outcome: Blank trimmed lines `continue`; malformed JSON writes JSON-RPC `-32700` `"Parse error: Invalid JSON"` and continues serving.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-017-1 (REQ-017)

- Method: Async unit test `mcp::tests::test_mcp_session_manager` and inspection of `SessionStream` (`L604-L631`) / `sse_handler` (`L634-L657`).
- Expected outcome: `McpSessionManager` tracks active sessions, routes events to matching channel, removes sessions, and spawns cleanup in `SessionStream::drop`.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-017-2 (REQ-017)

- Method: Source inspection of `src/mcp.rs::message_handler` (`L666-L689`) and `direct_post_handler` (`L692-L698`).
- Expected outcome: `message_handler` returns `404 Not Found` on missing/closed session for response-bearing requests and `202 Accepted` on delivery; `direct_post_handler` returns `204 No Content` for notifications.
- Execution: [Run 1](#run-1--2026-11-21)

## Execution evidence

### Run 1 — 2026-11-21

- Checked: REQ-013, REQ-014, REQ-015, REQ-016, REQ-017 / AC-013-1..AC-017-2 (commit `e95a3d4`, `rustc 1.92.0` / `cargo 1.92.0`)
- Command: `rtk proxy cargo test --offline` (exit code: `0`) + static inspection of `src/mcp.rs`, `src/main.rs`, `src/bin/vietcalendar-mcp.rs`
- Output excerpt:
  ```text
  running 23 tests
  ...
  test mcp::tests::test_mcp_initialize ... ok
  ...
  test mcp::tests::test_mcp_convert_solar_to_lunar_call ... ok
  test mcp::tests::test_mcp_convert_lunar_to_solar_call ... ok
  test mcp::tests::test_mcp_resources ... ok
  ...
  test mcp::tests::test_mcp_tools_list ... ok
  test mcp::tests::test_mcp_session_manager ... ok
  ...
  test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
  ```
- Result: AC-013-1 `PASS`, AC-013-2 `PASS`, AC-014-1 `PASS`, AC-014-2 `PASS`, AC-015-1 `PASS`, AC-015-2 `PASS`, AC-016-1 `PASS`, AC-016-2 `PASS`, AC-017-1 `PASS`, AC-017-2 `PASS` — all 6 MCP unit tests passed; `stdio` loop, HTTP SSE/POST handlers, and error paths verified by source inspection (end-to-end HTTP/stdio MCP harness remains a coverage gap).

## Real-world observations

No real-world confirmation has been recorded.
