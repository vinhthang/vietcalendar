# Design: Dynamic AI Agent Discovery & Synchronized Docs

Sources: [spec.md](./spec.md), [decisions.md](./decisions.md)

## Components

| Requirement / AC | Path and symbol | Responsibility / change |
| --- | --- | --- |
| REQ-043 / AC-043-1, AC-043-2 | `src/plugins/mcp_core.rs` (`build_initialize_instructions`, `McpCoreEngine::dispatch`) | Compute live `result.instructions` from wait-free `RegistrySnapshot` on `initialize`. |
| REQ-042 / AC-042-1, AC-042-2 | `src/plugins/http_discovery.rs` (`HttpDiscoveryPlugin`, `sanitize_origin`, `DiscoveryApiDoc`), `src/plugins/mod.rs` | Serve dynamic `/llms.txt`, `/llms-full.txt`, and `/.well-known/mcp.json` from `HttpRouterService` + optional `McpRegistryService`. |
| REQ-042, REQ-043 / AC-042-1, AC-043-1 | `src/profiles/mod.rs`, `config/cordis.yml`, `tests/integration_test.rs`, `llms.txt`, `README.md`, `docs/mcp_setup.md`, `AGENTS.md` | Wire `http_discovery`, verify `serve`/`http-only`/dispose E2E, add root `llms.txt`, refresh `README.md`, `docs/mcp_setup.md`, and `AGENTS.md`. |

## Interfaces and flow

- **Dynamic MCP `initialize.instructions` (`mcp_core.rs`):** On `method == "initialize"`, `McpCoreEngine::dispatch` loads `self.inner.snapshot.load()`, sorts active tool names & template URIs, and emits `"instructions"` explaining UTC+7 anchoring, niche tool groups, and resource URI patterns. Zero locks held on read.
- **Dynamic HTTP Discovery (`http_discovery.rs`):** `HttpDiscoveryPlugin` declares `requires::<dyn HttpRouterService>()` so it mounts in both `serve` and `http-only`. Request handlers inspect `HttpRouterService::current_openapi()` for active REST paths (excluding `/llms.txt`, `/llms-full.txt`, `/.well-known/mcp.json`) and call `ctx.try_inject::<dyn McpRegistryService>()` to gather active tools, resources, and URI templates via JSON-RPC `list` calls. Disposing `http_discovery` invokes its `RouteDisposer` via `ctx.on_dispose_sync`.

## Boundaries and recovery

- **Origin Header Sanitization (`sanitize_origin`):** Optional `config.base_url` overrides request headers when configured cleanly. Otherwise `X-Forwarded-Proto` is clamped strictly to `"http"` or `"https"`, and `Host` must be non-empty, `<= 253` ASCII chars matching `[A-Za-z0-9.:-\[\]]` with zero whitespace, CRLF, `/`, `)`, or control characters—falling back to `""` (root-relative links).
- **Zero-Panic & OpenAPI Safety:** Zero `.unwrap()`, `.expect()`, `panic!()`, or `RwLock` in production code; `DiscoveryApiDoc` documents `/llms.txt`, `/llms-full.txt`, and `/.well-known/mcp.json` without introducing `LunarDate`.

## Dependencies and unresolved design work

None.
