# Verification: Niche Lunar Calendar & Cultural Intelligence MCP/REST Plugins

Governing specification: [spec.md](./spec.md)

## Planned checks

### AC-036-1 (REQ-036)

- Method: Unit tests in `plugins::canchi_solar_vn` verifying day/month/year/hour Can Chi (`2000-01-01`, `1984-02-02`, `2024-02-10`), `23:30` Giờ Tý shift, Nạp Âm elements, and 24 solar terms for `2024` (`Lập Xuân`, `Xuân Phân`, `Hạ Chí`, `Đông Chí`).
- Expected outcome: Exact astronomical Can Chi & solar term matches with bilingual `(id, name_vi, name_en)` fields.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-036-2 (REQ-036)

- Method: Unit boundary tests in `plugins::canchi_solar_vn` for `hour >= 24`, `NaN`/`Infinity`/`±15.0` timezone, and invalid years.
- Expected outcome: All invalid calls return typed `Err(CalendarError)` without panicking.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-037-1 (REQ-037)

- Method: Unit tests in `plugins::astrology_vn` verifying BaZi Lập Xuân boundary (`2024-02-03` vs `2024-02-04`), Hidden Stems, Ten Gods, Five Elements, Đại Vận, and Tử Vi 12 palaces, Cục, 14 Chính Tinh, ~36 auxiliary stars, Tứ Hóa, Vòng Thái Tuế, Vòng Trường Sinh, Tuần/Triệt, leap-month split, and Đại Hạn.
- Expected outcome: Deterministic bilingual BaZi and Tử Vi Nam Phái Core charts matching reference fixtures.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-037-2 (REQ-037)

- Method: Unit boundary tests in `plugins::astrology_vn` for invalid dates, `hour >= 24`, `minute >= 60`, and invalid system/gender inputs.
- Expected outcome: Typed `Err(CalendarError)` returned cleanly with zero panics.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-038-1 (REQ-038)

- Method: Unit tests in `plugins::trach_nhat_vn` checking `2024-02-10` (`Trực Mãn`, `Sao Liễu`, Hoàng Đạo hours), Tam Tai/Kim Lâu/Hoang Ốc/Xung Tuổi for `1990`, and range search sorting.
- Expected outcome: Exact scores, taboo flags, age checks, and strict `(score DESC, date ASC)` ordering.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-038-2 (REQ-038)

- Method: Unit boundary tests in `plugins::trach_nhat_vn` for `end < start`, `> 366` days, and invalid `birth_year <= 0`.
- Expected outcome: Clean `Err(CalendarError)` rejection without panicking.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-039-1 (REQ-039)

- Method: Unit tests in `plugins::recurrence_vn` verifying `Day30Policy`, `LeapMonthPolicy` across `2023` & `2025`, vegetarian presets, and RFC 5545 `.ics` (`CRLF`, `<= 75` octet folding, `VALARM`).
- Expected outcome: Exact occurrences and spec-compliant iCalendar payload.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-039-2 (REQ-039)

- Method: Unit boundary tests in `plugins::recurrence_vn` for `> 50` year window, `start > end`, day `0`/`31`, and month `0`/`13`.
- Expected outcome: Typed `Err(CalendarError)` rejection without state mutation.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-040-1 (REQ-040)

- Method: Unit tests in `plugins::retail_vn` and `plugins::config_reloader` checking 2025 event resolution (`tat_nien_giao_thua -> 2025-01-28`, `ngay_via_than_tai -> 2025-02-07`, `trung_thu -> 2025-10-06`), milestones, and atomic `reload_from_disk` swap.
- Expected outcome: All 14 YAML events + monthly Mùng 1/Rằm resolve accurately and reload swaps `ArcSwap` atomically.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-040-2 (REQ-040)

- Method: Unit tests in `plugins::retail_vn::loader` and `plugins::config_reloader` passing unknown YAML keys, invalid day/month, duplicate IDs, negative offsets, empty bilingual strings, and `days > 366`.
- Expected outcome: Invalid queries/YAML reject cleanly while preserving prior active `ArcSwap` catalog.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-041-1 (REQ-041)

- Method: Integration test `test_niche_plugins_http_openapi_mcp_and_cascade_dispose` + `cargo bench --offline --bench calendar_bench` + `cargo mutants` + `arch-guard`.
- Expected outcome: All 10 new REST endpoints, OpenAPI JSON, 16 MCP tools, 3 resource templates, selective `kernel.dispose("astrology_vn")` cascade suspension, and 7 new Criterion benchmarks succeed with zero warnings and zero production `.unwrap()`.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-041-2 (REQ-041)

- Method: Negative HTTP `400` and MCP `-32602` checks in `test_niche_plugins_http_openapi_mcp_and_cascade_dispose` + profile boot verification across `serve`, `mcp-stdio`, and `http-only`.
- Expected outcome: Invalid REST inputs emit `HTTP 400`, invalid MCP args emit JSON-RPC `-32602`, and unmounted/disabled adapters expose zero dangling routes or tools.
- Execution: [Run 1](#run-1--2026-11-21)

## Execution evidence

### Run 1 — 2026-11-21

- Checked: REQ-036..REQ-041 (`cargo 1.92.0` / `rustc 1.92.0` offline Linux sandbox)
- Command: `RUSTFLAGS="-D warnings" cargo test --offline --all-targets && arch-guard check_boundaries.py . && cargo bench --offline --bench calendar_bench -- --warm-up-time 1 --measurement-time 2 && cargo mutants --timeout 60 -j 2 -C --offline -f src/plugins/retail_vn/loader.rs -f src/plugins/canchi_solar_vn/canchi.rs -f src/plugins/recurrence_vn/mod.rs -- --lib` (exit code: `0` test/bench/arch-guard; `cargo mutants` reported 2 defensive modulo survivors out of 112 viable mutants = `98.2%` caught)
- Output excerpt:
  ```text
  lib unit tests: ok. 67 passed; 0 failed; 0 ignored
  integration_test: ok. 7 passed; 0 failed; 0 ignored
  PROD_STYLE_AUDIT: CLEAN (zero .unwrap()/.expect()/panic!()/RwLock/wildcard imports in production src/)
  Arch Guard: Zero forbidden cross-boundary imports; Zero circular dependencies (CLEAN & PASSING)
  Criterion medians: canchi_full_breakdown=11.88µs, solar_terms_2024_cached=9.94µs, astrology_bazi_and_tuvi=226.27µs, trach_nhat_search_30d=1.91ms, recurrence_10yr_expand=395.28µs, recurrence_ics_render=60.63µs, retail_2025_year_lookup=152.84µs
  cargo mutants: 135 tested (110 caught, 22 unviable, 2 missed = 98.2% kill rate)
  ```
- Result: AC-036-1 `PASS`, AC-036-2 `PASS`, AC-037-1 `PASS`, AC-037-2 `PASS`, AC-038-1 `PASS`, AC-038-2 `PASS`, AC-039-1 `PASS`, AC-039-2 `PASS`, AC-040-1 `PASS`, AC-040-2 `PASS`, AC-041-1 `PASS`, AC-041-2 `PASS` — Full multi-target zero-warning test suite, architectural boundary audit, Criterion benchmarks, and mutation testing passed after all source edits; `cargo fmt`/`clippy` deferred to GitHub Actions CI as neither component is installed locally.

## Real-world observations

No real-world confirmation has been recorded.
