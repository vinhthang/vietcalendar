# Tasks: Niche Lunar Calendar & Cultural Intelligence MCP/REST Plugins

Sources: [spec.md](./spec.md), [design.md](./design.md)

## Task 1

**Define contract traits, DTOs, and implement shared `canchi_solar_vn` engine**

- [x] Complete the work and its applicable checks.
- Covers: REQ-036, REQ-041 / AC-036-1, AC-036-2, AC-041-1
- Files: `src/contracts/models.rs`, `src/contracts/canchi.rs`, `src/contracts/astrology.rs`, `src/contracts/trach_nhat.rs`, `src/contracts/recurrence.rs`, `src/contracts/retail.rs`, `src/contracts/mod.rs`, `src/plugins/canchi_solar_vn/mod.rs`, `src/plugins/canchi_solar_vn/canchi.rs`, `src/plugins/canchi_solar_vn/solar_terms.rs`, `src/plugins/mod.rs`
- Dependencies: None
- Blocked by: None
- Work: Write failing table-driven unit tests for day/month/year/hour Can Chi, 23:00 Tý rollover, Nạp Âm, 24 Tiết Khí in 2024, and invalid hour/timezone errors; define all five contract modules (`BilingualLabel`, service traits, DTOs) and implement `CanChiSolarPlugin` with bounded `moka` cache.
- Completion evidence: Contracts + 3 `canchi_solar_vn` unit tests passed under `-D warnings` and verified in [Run 1](./verification.md#run-1--2026-11-21).

## Task 2

**Implement Niche #1 `astrology_vn` (BaZi and Vietnamese Tử Vi Nam Phái Core)**

- [x] Complete the work and its applicable checks.
- Covers: REQ-037 / AC-037-1, AC-037-2
- Files: `src/plugins/astrology_vn/mod.rs`, `src/plugins/astrology_vn/bazi.rs`, `src/plugins/astrology_vn/tuvi.rs`, `src/plugins/mod.rs`
- Dependencies: Task 1
- Blocked by: None
- Work: Write failing table-driven tests for BaZi Lập Xuân splits (`2024-02-03` vs `2024-02-04`), Hidden Stems, Ten Gods, Five Elements, Đại Vận, and Tử Vi 12 palaces, Cục, 14 Chính Tinh, ~36 auxiliary stars, Tứ Hóa, Vòng Thái Tuế, Vòng Trường Sinh, Tuần/Triệt, leap-month split, and error bounds; implement `AstrologyPlugin`.
- Completion evidence: All 3 `astrology_vn` unit tests passed in [Run 1](./verification.md#run-1--2026-11-21).

## Task 3

**Implement Niche #2 `trach_nhat_vn` (Hoàng Đạo, 12 Trực, 28 Tú, Taboos & Age Checks)**

- [x] Complete the work and its applicable checks.
- Covers: REQ-038 / AC-038-1, AC-038-2
- Files: `src/plugins/trach_nhat_vn/mod.rs`, `src/plugins/trach_nhat_vn/rules.rs`, `src/plugins/mod.rs`
- Dependencies: Task 1, Task 2
- Blocked by: None
- Work: Write failing table-driven tests for `2024-02-10` (`Trực Mãn`, `Sao Liễu`), Hoàng Đạo hours, major taboos, Tam Tai/Kim Lâu/Hoang Ốc/Xung Tuổi checks, ranked search, and `> 366`-day guard; implement `TrachNhatPlugin`.
- Completion evidence: All 3 `trach_nhat_vn` unit tests passed in [Run 1](./verification.md#run-1--2026-11-21).

## Task 4

**Implement Niche #3 `recurrence_vn` (Stateless Lunar Recurrence & RFC 5545 `.ics` Export)**

- [x] Complete the work and its applicable checks.
- Covers: REQ-039 / AC-039-1, AC-039-2
- Files: `src/plugins/recurrence_vn/mod.rs`, `src/plugins/recurrence_vn/ics.rs`, `src/plugins/mod.rs`
- Dependencies: Task 3
- Blocked by: None
- Work: Write failing table-driven tests for `Day30Policy`, `LeapMonthPolicy` across `2023`/`2025`, vegetarian presets, `> 50`-year guard, and RFC 5545 CRLF + octet folding `.ics` output; implement `LunarRecurrencePlugin`.
- Completion evidence: All 3 `recurrence_vn` unit tests and 100% viable mutant kill check passed in [Run 1](./verification.md#run-1--2026-11-21).

## Task 5

**Implement Niche #4 `retail_vn`, `config/retail_events.yml`, and `ConfigReloaderPlugin` Hook**

- [x] Complete the work and its applicable checks.
- Covers: REQ-040 / AC-040-1, AC-040-2
- Files: `config/retail_events.yml`, `src/plugins/retail_vn/mod.rs`, `src/plugins/retail_vn/loader.rs`, `src/plugins/config_reloader.rs`, `src/plugins/mod.rs`
- Dependencies: Task 4
- Blocked by: None
- Work: Author `config/retail_events.yml` (14 peaks + monthly Mùng 1/Rằm); write failing table-driven tests for 2025 Chạp Day-30 fallback (`2025-01-28`), campaign milestones, `<= 366` guard, strict YAML validation, and fail-safe reload; implement `RetailCalendarPlugin` and hook `reload_from_disk`.
- Completion evidence: `retail_vn` (3 tests), `config_reloader` (4 tests), and 100% viable `loader.rs` mutant kill check passed in [Run 1](./verification.md#run-1--2026-11-21).

## Task 6

**Wire Thin HTTP/MCP Adapters, Toggleable Profiles, Integration Tests, Benchmarks & Docs**

- [x] Complete the work and its applicable checks.
- Covers: REQ-041 / AC-041-1, AC-041-2
- Files: `src/plugins/http_niches.rs`, `src/plugins/mcp_niches.rs`, `src/plugins/mod.rs`, `src/profiles/mod.rs`, `config/cordis.yml`, `tests/integration_test.rs`, `benches/calendar_bench.rs`, `README.md`, `AGENTS.md`, `docs/ARCHITECTURE_DECISIONS.md`
- Dependencies: Task 5
- Blocked by: None
- Work: Implement 5 HTTP/OpenAPI + 5 MCP niche plugins, register 15 factories in `build_default_factories` and `retail_vn` `config_base_dir` injection, update `config/cordis.yml`, expand E2E HTTP/OpenAPI/MCP/cascade-unmount tests and 7 Criterion microbenchmarks, update docs, and run final `cargo mutants` + `code-review` + `arch-guard` gates.
- Completion evidence: Full 67 lib + 7 integration test run, 13 Criterion benchmarks, `arch-guard` CLEAN, static zero-panic audit, and `cargo mutants` (`110/112`) verified in [Run 1](./verification.md#run-1--2026-11-21).

## Gate 2: traceability and readiness

| Requirement | AC | ADR or rationale | Design | Task | Planned verification | Status / reason |
| --- | --- | --- | --- | --- | --- | --- |
| REQ-036 | AC-036-1 | [ADR-011](./decisions.md#adr-011-shared-can-chi-substrate-and-decoupled-thin-plugin-architecture) | [Interfaces](./design.md#interfaces-and-flow) | [Task 1](#task-1) | [AC-036-1](./verification.md#ac-036-1-req-036) | PASS — Can Chi & 24 Tiết Khí parity mapped |
| REQ-036 | AC-036-2 | [ADR-012](./decisions.md#adr-012-wait-free-arcswapmoka-hot-path-stateless-recurrence-and-fail-safe-retail-catalog) | [Boundaries](./design.md#boundaries-and-recovery) | [Task 1](#task-1) | [AC-036-2](./verification.md#ac-036-2-req-036) | PASS — hour/timezone/year bounds mapped |
| REQ-037 | AC-037-1 | [ADR-011](./decisions.md#adr-011-shared-can-chi-substrate-and-decoupled-thin-plugin-architecture) | [Interfaces](./design.md#interfaces-and-flow) | [Task 2](#task-2) | [AC-037-1](./verification.md#ac-037-1-req-037) | PASS — BaZi & Tử Vi Nam Phái mapped |
| REQ-037 | AC-037-2 | [Rationale](./decisions.md#architectural-rationale) | [Boundaries](./design.md#boundaries-and-recovery) | [Task 2](#task-2) | [AC-037-2](./verification.md#ac-037-2-req-037) | PASS — birth input validation mapped |
| REQ-038 | AC-038-1 | [ADR-011](./decisions.md#adr-011-shared-can-chi-substrate-and-decoupled-thin-plugin-architecture) | [Interfaces](./design.md#interfaces-and-flow) | [Task 3](#task-3) | [AC-038-1](./verification.md#ac-038-1-req-038) | PASS — Trạch Nhật rules & ranking mapped |
| REQ-038 | AC-038-2 | [Rationale](./decisions.md#architectural-rationale) | [Boundaries](./design.md#boundaries-and-recovery) | [Task 3](#task-3) | [AC-038-2](./verification.md#ac-038-2-req-038) | PASS — 366-day & birth-year bounds mapped |
| REQ-039 | AC-039-1 | [ADR-012](./decisions.md#adr-012-wait-free-arcswapmoka-hot-path-stateless-recurrence-and-fail-safe-retail-catalog) | [Interfaces](./design.md#interfaces-and-flow) | [Task 4](#task-4) | [AC-039-1](./verification.md#ac-039-1-req-039) | PASS — recurrence policies & RFC 5545 `.ics` mapped |
| REQ-039 | AC-039-2 | [ADR-012](./decisions.md#adr-012-wait-free-arcswapmoka-hot-path-stateless-recurrence-and-fail-safe-retail-catalog) | [Boundaries](./design.md#boundaries-and-recovery) | [Task 4](#task-4) | [AC-039-2](./verification.md#ac-039-2-req-039) | PASS — 50-year horizon & lunar day/month guards mapped |
| REQ-040 | AC-040-1 | [ADR-012](./decisions.md#adr-012-wait-free-arcswapmoka-hot-path-stateless-recurrence-and-fail-safe-retail-catalog) | [Interfaces](./design.md#interfaces-and-flow) | [Task 5](#task-5) | [AC-040-1](./verification.md#ac-040-1-req-040) | PASS — 14 retail events & hot-reload mapped |
| REQ-040 | AC-040-2 | [ADR-012](./decisions.md#adr-012-wait-free-arcswapmoka-hot-path-stateless-recurrence-and-fail-safe-retail-catalog) | [Boundaries](./design.md#boundaries-and-recovery) | [Task 5](#task-5) | [AC-040-2](./verification.md#ac-040-2-req-040) | PASS — strict YAML validation & fail-safe preserve mapped |
| REQ-041 | AC-041-1 | [ADR-011](./decisions.md#adr-011-shared-can-chi-substrate-and-decoupled-thin-plugin-architecture) | [Components](./design.md#components) | [Task 1](#task-1), [Task 6](#task-6) | [AC-041-1](./verification.md#ac-041-1-req-041) | PASS — 10 REST, OpenAPI, 16 MCP tools, unmount & benches mapped |
| REQ-041 | AC-041-2 | [ADR-011](./decisions.md#adr-011-shared-can-chi-substrate-and-decoupled-thin-plugin-architecture) | [Boundaries](./design.md#boundaries-and-recovery) | [Task 6](#task-6) | [AC-041-2](./verification.md#ac-041-2-req-041) | PASS — HTTP 400, MCP `-32602`, YAML toggle mapped |

- Backward scope check: PASS — Tasks 1–6 cover `REQ-036..REQ-041` cleanly without touching `calendar_vn::validate_year`.
- Reference and catalog consistency: PASS — `REQ-036..REQ-041`, `ADR-011..ADR-012`, and `docs/specs/INDEX.md` are aligned.
