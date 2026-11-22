# Decisions: Dynamic AI Agent Discovery & Synchronized Docs

Governing specification: [spec.md](./spec.md)

## Architectural rationale

- Governs: REQ-042, REQ-043

Because VietCalendar composes domain and transport capabilities dynamically per YAML profile and supports live plugin hot-reload/unmount, static compiled `/llms.txt` or `initialize.instructions` strings would drift whenever a niche plugin is disabled in `config/cordis.yml` or disposed at runtime. Synthesizing discovery documents directly from wait-free `ArcSwap` snapshots (`HttpRouterService::current_openapi()` and optional `McpRegistryService` via `Context::try_inject` / `McpCorePlugin`) guarantees 100% live accuracy without blocking locks or violating `src/kernel/` domain isolation.

## ADR-013: Wait-Free Snapshot Agent Discovery (`http_discovery`) & Dynamic MCP `initialize.instructions`

- Governs: REQ-042, REQ-043
- State: Accepted
- Context and constraints: HTTP discovery endpoints must work identically in `serve` (REST + MCP enabled) and `http-only` (REST only) without failing mount when `McpRegistryService` is absent. Meanwhile, untrusted reverse-proxy headers (`Host`, `X-Forwarded-Proto`) must never inject bogus links or schemes into Markdown/JSON payloads.
- Viable options:
  - Option A: Add `HttpDiscoveryPlugin` (`http_discovery`) requiring only `HttpRouterService` and using `Context::try_inject::<dyn McpRegistryService>()` (or wait-free handle lookup) plus `sanitize_origin`, and generate `initialize.instructions` inside `McpCoreEngine` directly from its `ArcSwap<RegistrySnapshot>`, complemented by a repo-root `llms.txt` Markdown index for repository readers.
  - Option B: Embed static compile-time Markdown/JSON files into `HttpServerPlugin` / `OpenApiPlugin`.
- Decision: Select Option A. Keeping `http_discovery` as a decoupled Cordis plugin allows clean enabling in `serve`/`http-only`, omits HTTP overhead in `mcp-stdio`, dynamically tracks mounted plugins/tools, and validates HTTP origins strictly (`http`/`https` + strict ASCII host characters).
- Costs and risks: Calling `try_inject::<dyn McpRegistryService>()` in `http_discovery` handlers introduces an `ArcSwap` lookup only during infrequent agent discovery fetches, adding zero overhead to calendar/holidays hot paths. Mitigated cleanly by querying live kernel registry snapshot only inside discovery requests and sorting output deterministically.
- Rejected alternatives: Option B breaks when individual niche plugins or MCP are toggled off in `config/cordis.yml` / `cordis.patch.yml`.
- Affected components: `src/plugins/mcp_core.rs`, `src/plugins/http_discovery.rs`, `src/plugins/mod.rs`, `src/profiles/mod.rs`, `config/cordis.yml`, `llms.txt`, `README.md`, `docs/mcp_setup.md`, `AGENTS.md`.

## Gate 1: normative review

| Check | Status | Evidence / required correction |
| --- | --- | --- |
| Codebase grounding | PASS | Grounded against inspected `src/contracts/{http,mcp}.rs`, `src/plugins/{http_server,openapi,mcp_core}.rs`, `README.md`, and `docs/mcp_setup.md`. |
| Attributable intent and explicit scope | PASS | `REQ-042..REQ-043` trace directly to user's `llms.txt` + `README.md` request and approval; static hardcoding & `mcp-stdio` HTTP exposure explicitly excluded. |
| Observable requirements and meaningful acceptance criteria | PASS | Clear status codes, MIME types, dynamic unmount assertions, `http-only` `mcp_enabled: false`, header sanitization, and `initialize.instructions` checks. |
| Assumptions and owner-deferred questions are explicit | PASS | Origin header fallback (`""` -> root-relative paths) documented cleanly; zero `[PROVISIONAL]` items. |
| Architectural rationale respects requirements | PASS | `ADR-013` keeps kernel domain-free, handles `http-only` via `try_inject`, and prevents drift on live plugin toggles. |
| IDs, references, and revision route are consistent | PASS | Minted `REQ-042..REQ-043` and `ADR-013` cleanly after `REQ-041` / `ADR-012` without modifying frozen prior specs. |
