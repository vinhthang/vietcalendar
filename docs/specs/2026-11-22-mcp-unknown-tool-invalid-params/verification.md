# Verification: Unknown MCP Tool Returns Invalid Params (`-32602`)

Governing specification: [spec.md](./spec.md)

## Planned checks

### AC-044-1 (REQ-044)

- Method: Unit test `plugins::mcp_core::tests::test_mcp_core_protocol_dispatch_and_errors` + integration tests in `tests/integration_test.rs`.
- Expected outcome: Valid `tools/call` calls succeed and unknown top-level method returns `-32601`.
- Execution: [Run 1](#run-1--2026-11-22)

### AC-044-2 (REQ-044)

- Method: Unit test `plugins::mcp_core::tests::test_mcp_core_protocol_dispatch_and_errors` calling `tools/call` with `"name": "missing"`.
- Expected outcome: Returns JSON-RPC error code `-32602` with message `"Tool not found: missing"`.
- Execution: [Run 1](#run-1--2026-11-22)

## Execution evidence

### Run 1 — 2026-11-22

- Checked: REQ-044 / AC-044-1, AC-044-2 (`rustc 1.97.1` / `cargo 1.97.1`, `rustfmt 1.9.0`, `clippy 0.1.97`)
- Command: `cargo fmt --all -- --check && cargo clippy --offline --all-targets -- -D warnings && RUSTFLAGS="-D warnings" cargo test --offline --all-targets && python3 arch-guard/check_boundaries.py .` (exit code: `0`)
- Output excerpt:
  ```text
  RED baseline (test_mcp_core_protocol_dispatch_and_errors before dispatch fix):
    assertion `left == right` failed: left: -32601, right: -32602

  GREEN verification after -32602 fix:
    cargo clippy: No issues found
    plugins::mcp_core: ok. 3 passed; 0 failed
    src/lib.rs unit tests: ok. 72 passed; 0 failed; 0 ignored
    tests/integration_test.rs: ok. 8 passed; 0 failed; 0 ignored
    Arch Guard: CLEAN & PASSING
  ```
- Result: AC-044-1 `PASS`, AC-044-2 `PASS` — unknown `tools/call` tool names return JSON-RPC `-32602` while unknown top-level methods continue returning `-32601`.

## Real-world observations

No real-world confirmation has been recorded.
