# Verification: Non-Blocking Plugin Refactor and Debt Remediation

Governing specification: [spec.md](./spec.md)

## Planned checks

### AC-027-1 (REQ-027)

- Method: Unit test `plugins::calendar_vn::tests::test_exhaustive_15000_day_aa98_equivalence_and_roundtrip` across timezones `7.0`, `8.0`, and `-5.0`.
- Expected outcome: Zero mismatches across all 15,000 days (`1995-01-01`..`2035-12-31`); `lunar_month_days` reports exact `29` or `30` days.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-027-2 (REQ-027)

- Method: Unit test `plugins::calendar_vn::tests::test_strict_29_day_month_and_validation_boundaries`.
- Expected outcome: Invalid month `13`, day `30` on a 29-day *tháng thiếu*, invalid `is_leap_month`, and `NaN`/`15.0` timezone return `Err(CalendarError)` without rollover or panic.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-028-1 (REQ-028)

- Method: Unit tests `plugins::calendar_vn::tests::test_is_lunar_leap_year_accuracy`, `contracts::models::tests::test_date_month_year_serde`, and `test_lunar_date_serde`.
- Expected outcome: Leap-year (`2004`, `2020`, `2023`, `2025`, `2033` true; `2024` false) and Serde `"MM"`/`"mm"`/`leap` assertions pass.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-028-2 (REQ-028)

- Method: `rtk rg 'is_solar_leap|is_solar_holiday' src/ tests/ benches/` and filesystem verification that legacy `src/{calendar,services,handlers,mcp,models}.rs` no longer exist.
- Expected outcome: Zero matches (`exit=1`) and all 5 legacy root `.rs` files removed.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-029-1 (REQ-029)

- Method: Unit tests `plugins::holidays_vn::tests::test_official_2024_2025_2026_yaml_and_fallback`, `test_reversible_plugin_schedule_override_and_cache_invalidation`, and `contracts::models::tests::test_year_holiday_schedule_swapped_weekend`.
- Expected outcome: Official holidays return `true`, swapped working Saturdays (`2024-05-04`, `2025-04-26`, `2026-08-22`) return `false`, and disposer restores prior schedule + clears `moka` cache.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-029-2 (REQ-029)

- Method: Unit test `test_official_2024_2025_2026_yaml_and_fallback` and integration test `test_http_rest_endpoints_end_to_end` querying unconfigured year `2099`.
- Expected outcome: Only `Saturday` and `Sunday` return `true`; weekdays (`2099-01-01`) return `false`; `year_holidays(2099)` has zero entries.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-030-1 (REQ-030)

- Method: Integration tests `test_http_rest_endpoints_end_to_end` and `test_live_plugin_unmount_updates_router_openapi_and_mcp`.
- Expected outcome: Active routes return `200`, `/api-docs/openapi.json` omits `LunarDate`, CORS header is `*`, and disposing `http_holidays` yields `404` + OpenAPI removal.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-030-2 (REQ-030)

- Method: Integration test `test_http_rest_endpoints_end_to_end` (`dd=notanumber` and day `30` on 29-day month).
- Expected outcome: Both return `HTTP 400` with JSON `ErrorResponse`.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-031-1 (REQ-031)

- Method: Integration test `test_mcp_http_direct_post_tools_and_resources` and `vietcalendar-mcp` stdio CLI test.
- Expected outcome: `get_year_holidays(2026)` returns all 13 entries and `working_weekends: ["2026-08-22"]`; `resources/templates/list` returns `calendar://holidays/{year}`.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-031-2 (REQ-031)

- Method: Integration test `test_mcp_http_direct_post_tools_and_resources` passing day `30` on a 29-day month to `convert_lunar_to_solar`.
- Expected outcome: Returns JSON-RPC `-32602` without rollover or truncation.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-032-1 (REQ-032)

- Method: Integration tests `test_mcp_http_direct_post_tools_and_resources` and `test_mcp_sse_session_notification_404_and_backpressure_503`.
- Expected outcome: Initial `endpoint` event emitted, `202`/`200`/`204` semantics hold, and stream drop removes session synchronously.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-032-2 (REQ-032)

- Method: Integration test `test_mcp_sse_session_notification_404_and_backpressure_503` and unit test `test_session_manager_bounded_try_send_and_sync_drop`.
- Expected outcome: Unknown `sessionId` notification returns `404 Not Found`; full bounded SSE buffer returns `503 Service Unavailable`.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-033-1 (REQ-033)

- Method: Run `RUSTFLAGS="-D warnings" cargo test --offline --all-targets`, Python scan for `.unwrap(` / `.expect(` in production `src/`, inspect `.gitignore` and removed `conf/`.
- Expected outcome: Zero compiler warnings, `prod_unwrap_count = 0`, clean `.gitignore`, and `conf/` absent.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-033-2 (REQ-033)

- Method: Unit tests `test_telemetry_mounts_without_otlp_collector` and `test_bind_failure_returns_kernel_error_without_panic`.
- Expected outcome: `TelemetryPlugin` skips OTLP collector connection when unconfigured and `HttpServerPlugin` returns `Err(KernelError)` on bind error without panicking.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-034-1 (REQ-034)

- Method: Inspection of `Dockerfile` contents against actual Cargo targets (`src/lib.rs`, `benches/`, `config/`).
- Expected outcome: Builder stub matches Cargo targets and runtime stage copies `config/` to `/etc/vietcalendar/config` with `VIETCALENDAR_CONFIG_DIR` set and `USER appuser`.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-034-2 (REQ-034)

- Method: Inspection of `Dockerfile` cache-invalidation `touch` command and compatibility symlink.
- Expected outcome: `RUN touch ...` precedes final `cargo build --release` and `/usr/local/bin/vietcalendar-rs` symlink is preserved.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-035-1 (REQ-035)

- Method: Execute `rtk proxy cargo bench --offline --bench calendar_bench -- --warm-up-time 1 --measurement-time 2`.
- Expected outcome: All 6 benchmark groups in `benches/calendar_bench.rs` execute and exit `0`.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-035-2 (REQ-035)

- Method: Verify `cargo bench --offline` succeeds inside sandbox without network access or modifying `config/`.
- Expected outcome: Benchmark succeeds offline and `config/` remains unmodified.
- Execution: [Run 1](#run-1--2026-11-21)

## Execution evidence

### Run 1 — 2026-11-21

- Checked: REQ-027..REQ-035 / AC-027-1..AC-035-2 (`rustc 1.92.0` / `cargo 1.92.0`)
- Command: `RUSTFLAGS="-D warnings" rtk proxy cargo test --offline --all-targets`, `rtk proxy cargo bench --offline --bench calendar_bench -- --warm-up-time 1 --measurement-time 2`, `printf '...' | rtk proxy cargo run --offline --quiet --bin vietcalendar-mcp`, plus `.unwrap()`/legacy audits
- Output excerpt:
  ```text
  src/lib.rs unit tests: 23 passed; 0 failed; 0 ignored
  tests/integration_test.rs: 6 passed; 0 failed; 0 ignored
  prod_unwrap_count = 0 | legacy_rg_exit=1

  vietcalendar-mcp stdio:
    initialize -> protocolVersion "2024-11-05", serverInfo.version "0.2.0"
    get_year_holidays(2026) -> total_holidays=13, working_weekends=["2026-08-22"]

  Criterion microbenchmarks:
    calendar_to_lunar_warm   time: [459.08 ns 459.36 ns 459.65 ns]
    calendar_to_solar_warm   time: [244.63 ns 244.89 ns 245.23 ns]
    calendar_lunar_span_cold time: [6.2781 µs 6.2839 µs 6.2917 µs]
    holiday_is_holiday_warm  time: [216.36 ns 216.83 ns 217.49 ns]
    http_parse_ymd_ascii     time: [19.327 ns 19.383 ns 19.450 ns]
    kernel_wait_free_inject  time: [58.612 ns 58.816 ns 59.155 ns]
  ```
- Result: AC-027-1 `PASS`, AC-027-2 `PASS`, AC-028-1 `PASS`, AC-028-2 `PASS`, AC-029-1 `PASS`, AC-029-2 `PASS`, AC-030-1 `PASS`, AC-030-2 `PASS`, AC-031-1 `PASS`, AC-031-2 `PASS`, AC-032-1 `PASS`, AC-032-2 `PASS`, AC-033-1 `PASS`, AC-033-2 `PASS`, AC-034-1 `PASS`, AC-034-2 `PASS`, AC-035-1 `PASS`, AC-035-2 `PASS` — all 29 unit/integration tests, 6 Criterion microbenchmarks, stdio CLI, and static audits passed with zero warnings.

## Real-world observations

No real-world confirmation has been recorded.
