# Decisions: Non-Blocking Plugin Refactor and Debt Remediation

Governing specification: [spec.md](./spec.md)

## Architectural rationale

- Governs: REQ-027, REQ-028, REQ-029, REQ-030, REQ-031, REQ-032, REQ-033, REQ-034, REQ-035

Splitting monolithic modules (`src/{calendar,services,handlers,mcp,models}.rs`) into independent vertical plugins over `src/contracts/` traits and the Cordis kernel ([cordis-plugin-kernel](../2026-11-21-cordis-plugin-kernel/spec.md)) eliminates cross-layer duplication (`TD-01`), unifies HTTP/MCP validation (`TD-03`, `TD-06`), enforces non-blocking concurrency (`TD-02`, `TD-07`), and removes legacy migration noise (`TD-04`, `TD-08`). `REQ-030`–`REQ-035` follow cleanly from `ADR-007`, `ADR-008`, and the two domain/performance choices captured in `ADR-009` and `ADR-010`.

## ADR-009: Immutable LunarSpan Cache with Moka and ArcSwap Registries

- Governs: REQ-027, REQ-028, REQ-030, REQ-031, REQ-032, REQ-035
- State: Accepted
- Context and constraints: Baseline `services.rs` wrapped `HashMap<i32, HashSet<NaiveDate>>` in a blocking `std::sync::RwLock` and deep-cloned the entire `HashSet` per holiday request, while `calendar.rs` repeated trigonometric Jean Meeus AA98 evaluations on every date conversion and `mcp.rs` held an async `RwLock` + unbounded SSE channels.
- Viable options: Option A — Precompute immutable `Arc<LunarSpan>` (29/30-day lunar month table per year/timezone) and immutable `Arc<YearHolidaySchedule>` backed by bounded `moka::sync::Cache`, paired with `ArcSwap` wait-free snapshots for `HttpRouterService`, `McpRegistryService`, and bounded SSE `McpSessionManager`; vs. Option B — Replace `std::sync::RwLock` with `tokio::sync::RwLock` over mutable `HashMap`s without precomputing `LunarSpan`.
- Decision: Option A — Immutable `Arc<LunarSpan>` + `moka` W-TinyLFU caches + `ArcSwap` registries + bounded SSE `mpsc` channels.
- Costs and risks: Eliminates lock contention and per-request set cloning completely on warm paths, enforces O(1) exact 29/30-day lunar validation (`TD-03`), bounds memory against arbitrary year/timezone floods (`TD-02`), and prevents slow SSE consumers from exhausting memory (`TD-07`). Cold misses construct an immutable `LunarSpan` once (~12 new moons) and require explicit `moka` cache invalidation when holiday YAML files reload.
- Rejected alternatives: Option B leaves `tokio::sync::RwLock` write contention, fails to bound memory growth under arbitrary query years, and retains repeated Meeus trigonometric cost per conversion.
- Affected components: `src/plugins/calendar_vn/`, `src/plugins/holidays_vn/`, `src/plugins/http_server.rs`, `src/plugins/mcp_core.rs`, `src/plugins/mcp_http.rs`.

## ADR-010: Data-Driven Per-Year YAML Holiday Tables with `working_weekends` Overrides

- Governs: REQ-029, REQ-031, REQ-033, REQ-034
- State: Accepted
- Context and constraints: Vietnam's annual public holiday schedule varies year-to-year by government (MOLISA / Ministry of Home Affairs) decree—adding bridge days and swapping regular weekdays with compensatory working Saturdays (*ngày đi làm bù*, e.g., `2024-05-04`, `2025-04-26`, `2026-08-22`), which static algorithmic formulas cannot predict accurately (`TD-05`).
- Viable options: Option A — Load each year's exact holiday dates and `working_weekends` solely from `config/holidays/<year>.yml` (plus reversible Cordis plugin schedules), falling back to weekends-only (`Sat`/`Sun`) when a year is unconfigured; vs. Option B — Maintain algorithmic statutory rules plus per-year patch deltas.
- Decision: Option A — Pure data-driven per-year YAML schedules (`2024.yml`, `2025.yml`, `2026.yml` shipped) with weekends-only fallback for unconfigured years.
- Costs and risks: Guarantees 100% fidelity with real-world government decrees including swapped working Saturdays (`false` on `working_weekends`), removes hardcoded pre-2021 assumptions (`TD-05`), and supports zero-downtime live schedule additions via YAML hot-reload. Unconfigured historical or far-future years deliberately report only `Sat`/`Sun` until a `<year>.yml` file or plugin schedule is supplied.
- Rejected alternatives: Option B requires reconciling conflicting statutory cutoffs (`2019` vs `2021` Labor Code) and still cannot anticipate ad-hoc MOLISA bridge-day decrees without external overrides.
- Affected components: `config/holidays/*.yml`, `src/plugins/holidays_vn/`, `src/plugins/config_reloader.rs`, `Dockerfile`.

## Gate 1: normative review

| Check | Status | Evidence / required correction |
| --- | --- | --- |
| Codebase grounding | PASS | Grounded in `src/{calendar,models,services,handlers,mcp,main}.rs`, `Cargo.toml`, `Dockerfile`, and `docs/PROJECT_ANALYSIS.md` (`TD-01`–`TD-08`). |
| Attributable intent and explicit scope | PASS | `REQ-027`..`REQ-035` trace directly to `/grill-me` decisions and `TD-01`–`TD-08`; legacy internal API removal and non-shipped pre-2024 holiday YAML files documented in `Non-goals`. |
| Observable requirements and meaningful acceptance criteria | PASS | `AC-027-1`..`AC-035-2` cover 15,000-day AA98 equivalence, day-30 *tháng thiếu* rejection, 2024–2026 MOLISA schedules + working weekends, JSON extractor errors, live route/tool unmount, `503`/`404` SSE bounds, and `cargo bench`. |
| Assumptions and owner-deferred questions are explicit | PASS | Multi-REQ parent activation scope documented under `Assumptions`; zero `[PROVISIONAL]` or `TODO` markers. |
| Architectural rationale respects requirements | PASS | `ADR-009` and `ADR-010` compare genuine design options and govern `REQ-027`..`REQ-035`. |
| IDs, references, and revision route are consistent | PASS | Linked delta `REQ-027..REQ-035` and `ADR-009, ADR-010` match `docs/specs/INDEX.md`; parent `#req-NNN-*` anchors verified. |
