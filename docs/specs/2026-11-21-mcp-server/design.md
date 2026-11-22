# Design: Model Context Protocol (MCP) Server and Dual Transports

Sources: [spec.md](./spec.md), [decisions.md](./decisions.md)

## Components

| Requirement / AC | Path and symbol | Responsibility / change |
| --- | --- | --- |
| REQ-013 / AC-013-1, AC-013-2 | `src/mcp.rs::JsonRpcRequest`, `JsonRpcResponse`, `JsonRpcError`, `handle_request` | JSON-RPC `2.0` dispatch (`initialize` `2024-11-05`, `ping`, `notifications/initialized`, error codes `-32700`/`-32601`/`-32602`). |
| REQ-014 / AC-014-1, AC-014-2 | `src/mcp.rs::handle_request`, `parse_solar_input`, `HolidayInfo`, `compute_holiday_list_for_year` | Registers and executes 5 MCP tools (`get_today_lunar`, `convert_solar_to_lunar`, `convert_lunar_to_solar`, `check_vietnam_holiday`, `get_year_holidays`). |
| REQ-015 / AC-015-1, AC-015-2 | `src/mcp.rs::handle_request` (`resources/list`, `resources/read`) | Advertises `calendar://today` and `calendar://holidays/2026`; resolves live `calendar://today` and `calendar://holidays/{year}` JSON payloads. |
| REQ-016 / AC-016-1, AC-016-2 | `src/mcp.rs::run_stdio_server`, `src/main.rs::Commands::Mcp`, `src/bin/vietcalendar-mcp.rs::main` | Line-delimited JSON-RPC `stdin`/`stdout` transport loop. |
| REQ-017 / AC-017-1, AC-017-2 | `src/mcp.rs::McpSessionManager`, `SessionStream`, `sse_handler`, `MessageQuery`, `message_handler`, `direct_post_handler`; `src/main.rs` | Stateful SSE streaming (`GET /mcp/sse` + `POST /mcp/message?sessionId=...`) and direct HTTP POST (`POST /mcp`, `POST /mcp/sse`). |

## Interfaces and flow

- **Core dispatcher (`src/mcp.rs`):** `pub fn handle_request(req: JsonRpcRequest) -> Option<JsonRpcResponse>` returns `Some(response)` for requests and `None` for `"notifications/initialized"`.
- **Tool execution:** `get_today_lunar`, `convert_solar_to_lunar`, `convert_lunar_to_solar`, and `check_vietnam_holiday` call `services::{to_lunar, to_solar, is_vietnam_holiday}`; `get_year_holidays` and `calendar://holidays/{year}` call `compute_holiday_list_for_year(year, tz)`.
- **Remote HTTP transports (`src/main.rs` -> `src/mcp.rs`):**
  - `GET /mcp/sse` generates UUID `session_id`, sends `event: endpoint` (`/mcp/message?sessionId=<uuid>`), registers `UnboundedSender<Event>` in `McpSessionManager`, and returns `Sse<SessionStream>` with `KeepAlive::default()`. Dropping `SessionStream` spawns async removal.
  - `POST /mcp/message?sessionId=...` pushes `event: message` (`202 Accepted`) or `404 Not Found` if `session_id` is absent/closed.
  - `POST /mcp` and `POST /mcp/sse` invoke `direct_post_handler` returning `Json(resp)` (`200 OK`) or `204 No Content`.

## Boundaries and recovery

- **Stdio error resilience:** `run_stdio_server` skips whitespace-only lines and emits JSON-RPC `-32700` on invalid JSON without crashing.
- **Known baseline quirks (`TD-01`, `TD-03`, `TD-07`):** `compute_holiday_list_for_year` omits weekend compensatory days (*nghỉ bù*) and deduplicates same-day events by date only; MCP `convert_lunar_to_solar` does not reject out-of-range `day`/`month` prior to `to_solar`; `message_handler` returns `202 Accepted` on notifications without validating `session_id`; `McpSessionManager` uses `mpsc::unbounded_channel`.

## Dependencies and unresolved design work

None.
