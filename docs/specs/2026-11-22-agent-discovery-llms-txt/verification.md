# Verification: Dynamic AI Agent Discovery & Synchronized Docs

Governing specification: [spec.md](./spec.md)

## Planned checks

### AC-042-1 (REQ-042)

- Method: Unit tests in `plugins::http_discovery::tests` and integration test `test_agent_discovery_llms_txt_and_well_known_mcp`.
- Expected outcome: `/llms.txt`, `/llms-full.txt`, and `/.well-known/mcp.json` dynamically reflect mounted routes and MCP tools/templates and update immediately on `mcp_tools` or `astrology_vn` disposal.
- Execution: [Run 1](#run-1--2026-11-22)

### AC-042-2 (REQ-042)

- Method: Unit tests `test_sanitize_origin_rejects_malicious_headers` + `http-only` / `mcp_core` disposal integration checks.
- Expected outcome: `http-only` yields `mcp_enabled: false` and `tools: []`; untrusted headers fall back to safe relative paths.
- Execution: [Run 1](#run-1--2026-11-22)

### AC-043-1 (REQ-043)

- Method: Unit test `plugins::mcp_core::tests::test_initialize_includes_dynamic_live_instructions`, E2E `initialize` check, and documentation audit across `llms.txt`, `README.md`, `docs/mcp_setup.md`, and `AGENTS.md`.
- Expected outcome: `initialize` returns dynamic `instructions` tracking live tool/template changes; docs cover all 16 tools, 3 templates, discovery routes, and `ADR-013`.
- Execution: [Run 1](#run-1--2026-11-22)

### AC-043-2 (REQ-043)

- Method: Empty-registry `initialize` test in `mcp_core` and existing OpenAPI check (`openapi["components"]["schemas"]["LunarDate"].is_null()`).
- Expected outcome: Empty registry still emits valid `instructions` without error; OpenAPI preserves `LunarDate` omission invariant.
- Execution: [Run 1](#run-1--2026-11-22)

## Execution evidence

### Run 1 — 2026-11-22

- Checked: REQ-042, REQ-043 (`cargo 1.92.0` / `rustc 1.92.0` offline Linux sandbox)
- Command: `RUSTFLAGS="-D warnings" cargo test --offline --all-targets && arch-guard check_boundaries.py . && cargo mutants --timeout 60 -j 2 -C --offline -f src/plugins/http_discovery.rs -- --lib` (exit code: `0` test/arch-guard; `cargo mutants` caught `21/25` viable mutants = `84.0%`)
- Output excerpt:
  ```text
  lib unit tests: ok. 70 passed; 0 failed; 0 ignored
  integration_test: ok. 8 passed; 0 failed; 0 ignored
  PROD_STYLE_AUDIT: CLEAN (zero .unwrap()/.expect()/panic!()/RwLock/wildcard imports in production src/)
  Arch Guard: CLEAN & PASSING
  cargo mutants (http_discovery.rs): 36 tested (21 caught, 11 unviable, 4 missed = 84.0% kill rate)
  ```
- Result: AC-042-1 `PASS`, AC-042-2 `PASS`, AC-043-1 `PASS`, AC-043-2 `PASS` — Dynamic `/llms.txt`, `/llms-full.txt`, `/.well-known/mcp.json`, dynamic MCP `initialize.instructions`, root `llms.txt`, `README.md`, `docs/mcp_setup.md`, and `AGENTS.md` all verified cleanly.

## Real-world observations

No real-world confirmation has been recorded.
