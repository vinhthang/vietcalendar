# Dynamic AI Agent Discovery (`llms.txt`, `/.well-known/mcp.json`, MCP Instructions) & Synchronized Docs

- Feature ID: 2026-11-22-agent-discovery-llms-txt
- Type: enhancement
- Status: In-Testing
- First entered In-Testing: 2026-11-22
- Related specifications: None

## Problem and source intent

User request:
- "now what is the mcp server look like, does it have llm.txt to advertise for agent bot? and also, the readme file?"
- Followed by explicit approval: "yes do all of them" — add dynamic `GET /llms.txt`, `GET /llms-full.txt`, `GET /.well-known/mcp.json`, dynamic MCP `initialize.instructions`, repo-root `llms.txt`, and refresh `README.md` and `docs/mcp_setup.md` so LLM agents and developers see accurate live capabilities, tools, templates, and REST routes.

## Repository grounding

- HTTP router & OpenAPI composition: [`src/contracts/http.rs`](../../../src/contracts/http.rs), [`src/plugins/http_server.rs`](../../../src/plugins/http_server.rs), [`src/plugins/openapi.rs`](../../../src/plugins/openapi.rs).
- Wait-free MCP dispatcher & live registry: [`src/contracts/mcp.rs`](../../../src/contracts/mcp.rs), [`src/plugins/mcp_core.rs`](../../../src/plugins/mcp_core.rs).
- Profile & factory registration: [`src/profiles/mod.rs`](../../../src/profiles/mod.rs), [`config/cordis.yml`](../../../config/cordis.yml).
- Documentation files: [`README.md`](../../../README.md), [`docs/mcp_setup.md`](../../../docs/mcp_setup.md), [`AGENTS.md`](../../../AGENTS.md).

## Non-goals

- Static compile-time route/tool lists baked inside binary binaries when runtime plugins can be toggled.
- Changing existing MCP tool schemas or removing OpenAPI `LunarDate` omission guarantees (`REQ-030`).
- Enabling HTTP routes in the `mcp-stdio` profile.

## Requirements

### REQ-042: Dynamic Wait-Free HTTP Agent Discovery (`http_discovery`)

- Source intent: User request to advertise the server's capabilities and MCP/REST endpoints to agent bots (`llms.txt`, `llms-full.txt`, `/.well-known/mcp.json`).
- Statement: WHEN `HttpDiscoveryPlugin` (`http_discovery`) is mounted atop `HttpRouterService` and queried at `GET /llms.txt`, `GET /llms-full.txt`, or `GET /.well-known/mcp.json`, IT SHALL dynamically render discovery payloads strictly reflecting currently mounted REST paths (`current_openapi()`) and live MCP tools, resources, and URI templates (`McpRegistryService` via `try_inject`), sanitizing request/config origins against header injection and reverting cleanly on fiber disposal.
- AC-042-1 — Success: `/llms.txt` and `/llms-full.txt` return `HTTP 200` `text/markdown; charset=utf-8` (with full JSON input schemas in `/llms-full.txt`), `/.well-known/mcp.json` returns `HTTP 200` `application/json` with `mcp_enabled`, `protocol_version: "2024-11-05"`, transport endpoints (`/mcp`, `/mcp/sse`), and live tools/templates; unmounting an upstream plugin (`mcp_tools` or `astrology_vn`) immediately removes its tools and routes from all three discovery responses.
- AC-042-2 — Boundary/failure: In `http-only` mode (or after `mcp_core` disposal), `/.well-known/mcp.json` reports `mcp_enabled: false` with empty tool lists and `/llms.txt` omits the MCP transport section cleanly; untrusted `Host` or `X-Forwarded-Proto` headers containing CRLF, whitespace, path slashes, or non-HTTP schemes are sanitized to safe relative URLs (`""`).

### REQ-043: Dynamic MCP `initialize.instructions`, Repo-Root `llms.txt` & Synchronized Agent Documentation

- Source intent: User request asking for MCP inspection, `llms.txt` advertisement, and `README.md` / `docs/mcp_setup.md` updates.
- Statement: WHEN an MCP client sends `initialize` to `McpCorePlugin`, THE server SHALL emit a dynamic `result.instructions` string reflecting currently active tools and resource templates from its wait-free `ArcSwap` snapshot; AND THE repository SHALL ship a root `llms.txt` alongside synchronized `README.md`, `docs/mcp_setup.md`, and `AGENTS.md` covering all 28 registered plugins, 16 MCP tools, 3 resource templates, niche REST examples, and discovery endpoints.
- AC-043-1 — Success: `initialize` returns non-empty `result.instructions` listing all active tools and URI templates and immediately updates when a tool is registered or disposed; repo-root `llms.txt`, `README.md`, `docs/mcp_setup.md`, and `AGENTS.md` document all active REST routes, 16 MCP tools, 3 URI templates, `/llms.txt`, `/llms-full.txt`, `/.well-known/mcp.json`, and `ADR-013`.
- AC-043-2 — Boundary/failure: When zero MCP tools/templates are registered (e.g. after `mcp_tools` disposal or empty registry), `initialize` still returns valid `result.instructions` (guidance without panics or broken JSON) and `http_discovery` schemas do not expose internal `LunarDate` OpenAPI regression (`REQ-030`).

## Assumptions and owner-deferred questions

- `/.well-known/mcp.json` omits absolute hostname prefixes when `Host` / `X-Forwarded-Proto` headers fail validation or are absent, falling back safely to root-relative paths (`/mcp`, `/mcp/sse`).
- No `[PROVISIONAL]` requirements remain open.

## Approvals and lifecycle history

- `2026-11-22`: Initial specification authored from explicit user approval ("yes do all of them") to add dynamic `llms.txt`, `llms-full.txt`, `/.well-known/mcp.json`, MCP `initialize.instructions`, repo-root `llms.txt`, and documentation updates.
- `2026-11-22`: `In-Progress -> In-Testing` — Tasks 1–3 completed; all 70 lib unit tests, 8 integration tests (`0` warnings), static/`arch-guard` audits, and `http_discovery` mutation testing (`21/25` viable = `84.0%` caught) verified in [Run 1](./verification.md#run-1--2026-11-22).
