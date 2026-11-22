# Decisions: Niche Lunar Calendar & Cultural Intelligence MCP/REST Plugins

Governing specification: [spec.md](./spec.md)

## Architectural rationale

- Governs: REQ-036, REQ-037, REQ-038, REQ-039, REQ-040, REQ-041

The Cordis plugin kernel enforces strict zero-domain separation in `src/kernel/` and interface-only communication via `src/contracts/`. Modeling Can Chi + 24 Solar Terms as a dedicated shared contract (`CanChiSolarService`) with one pure engine plugin (`canchi_solar_vn`), four independent domain plugins (`astrology_vn`, `trach_nhat_vn`, `recurrence_vn`, `retail_vn`), and five thin adapter pairs (HTTP/OpenAPI + MCP) preserves independent toggling in `config/cordis.yml`, automatic LIFO unmount cascades when a core plugin is disposed, and wait-free read performance across all REST and MCP paths.

## ADR-011: Shared Can Chi Substrate and Decoupled Thin Plugin Architecture

- Governs: REQ-036, REQ-037, REQ-038, REQ-041
- State: Accepted
- Context and constraints: Can Chi (`day`, `month`, `year`, `hour`, *Nạp Âm*) and 24 solar terms (*Tiết Khí*) are required identically by BaZi, Tử Vi, Trạch Nhật, and direct LLM queries. Kernel architecture forbids cross-plugin tight coupling in `src/kernel/`, and operators need per-niche enable/disable control in `config/cordis.yml` across `serve`, `mcp-stdio`, and `http-only` profiles without duplicating HTTP/MCP adapters or astronomical math.
- Viable options:
  - Option A: Extract `CanChiSolarService` (`canchi_solar_vn`) as a shared Cordis service contract and pair each niche (`canchi_solar_vn`, `astrology_vn`, `trach_nhat_vn`, `recurrence_vn`, `retail_vn`) with thin optional HTTP/OpenAPI and MCP plugins (`http_niches.rs`, `mcp_niches.rs`), using structured `(id, name_vi, name_en)` bilingual DTOs.
  - Option B: Inline Can Chi / Tiết Khí math inside each niche module and mount a single monolithic `http_niche_all` / `mcp_niche_all` adapter plugin.
- Decision: Select Option A. Each domain module injects `Arc<dyn CalendarService>` and (where needed) `Arc<dyn CanChiSolarService>` through `ctx.inject`, registers a typed trait service (`AstrologyService`, `TrachNhatService`, `LunarRecurrenceService`, `RetailCalendarService`), and delegates transport mapping to per-niche HTTP and MCP plugins.
- Costs and risks: Increases the plugin catalog by 15 named plugin factories (`5` domain + `5` HTTP + `5` MCP), requiring explicit ordering in `config/cordis.yml`. Mitigated cleanly via grouping transport plugins into `http_niches.rs` and `mcp_niches.rs` and declaring deterministic kernel fiber dependencies via `ctx.inject`.
- Rejected alternatives: Option B violates DRY across Tiết Khí root-finding and makes per-niche enable/disable in `config/cordis.yml` impossible without recompilation.
- Affected components: `src/contracts/{mod,canchi,astrology,trach_nhat,recurrence,retail}.rs`, `src/plugins/{mod,canchi_solar_vn,astrology_vn,trach_nhat_vn,recurrence_vn,retail_vn,http_niches,mcp_niches}`, `src/profiles/mod.rs`, `config/cordis.yml`.

## ADR-012: Wait-Free ArcSwap/Moka Hot Path, Stateless Recurrence, and Fail-Safe Retail Catalog

- Governs: REQ-036, REQ-039, REQ-040, REQ-041
- State: Accepted
- Context and constraints: HTTP REST and MCP handlers execute concurrently under Tokio and must maintain wait-free reads (`ArcSwap` / bounded `moka`), zero runtime `.unwrap()`/panics, and hot-reload atomicity without introducing external database dependencies or unbounded range computations.
- Viable options:
  - Option A: Cache solar terms per year in bounded `moka::sync::Cache`, store parsed & validated `config/retail_events.yml` inside `ArcSwap<RetailCatalog>` reloaded via `ConfigReloaderPlugin::reload_from_disk`, keep `recurrence_vn` strictly stateless (bounded to `<= 50` years / `366`-day search windows), and apply explicit `Option::get` checked array indexing everywhere.
  - Option B: Persist user recurrence schedules in an embedded SQLite/JSON store and recompute 24 solar terms uncached on every request inside `std::sync::RwLock` maps.
- Decision: Select Option A. Stateless computation guarantees horizontal scalability and zero disk state corruption; bounded `moka` (`YEAR_CACHE`) eliminates repeated Meeus AA98 bisection; `ArcSwap<RetailCatalog>` mirrors `VietnamHolidaysPlugin`'s fail-safe staging/commit hot-reload pattern.
- Costs and risks: Clients wishing to store recurring events persistently must import generated JSON occurrences or RFC 5545 `.ics` feeds into their own calendar app. Mitigated directly by compliant `.ics` (`CRLF`, `<= 75` octet folding, `VALARM`) generation in `recurrence_vn`.
- Rejected alternatives: Option B introduces lock contention, state migration overhead, and violates the stateless microservice contract settled in planning.
- Affected components: `src/plugins/canchi_solar_vn/solar_terms.rs`, `src/plugins/recurrence_vn/{mod,ics}.rs`, `src/plugins/retail_vn/{mod,loader}.rs`, `src/plugins/config_reloader.rs`, `config/retail_events.yml`.

## Gate 1: normative review

| Check | Status | Evidence / required correction |
| --- | --- | --- |
| Codebase grounding | PASS | Grounded against inspected `src/kernel/mod.rs`, `src/contracts/*`, `src/plugins/{calendar_vn,config_reloader}.rs`, `src/profiles/mod.rs`, and `config/cordis.yml`. |
| Attributable intent and explicit scope | PASS | `REQ-036..REQ-041` trace directly to user niche MCP prompt and `/grill-me` decisions; exclusions (`SQLite`, niche #5 tides/agri, `validate_year` changes) stated in Non-goals. |
| Observable requirements and meaningful acceptance criteria | PASS | Each `REQ-036..REQ-041` specifies verifiable astronomical/date fixtures, scoring/sorting contracts, and typed error boundaries (`<= 366` days, `<= 50` years, strict YAML). |
| Assumptions and owner-deferred questions are explicit | PASS | `23:00` *Giờ Tý* default, Nam Phái Core ~50-star boundary, and `validate_year` preservation documented; zero `[PROVISIONAL]` items remain. |
| Architectural rationale respects requirements | PASS | `ADR-011` and `ADR-012` satisfy decoupled Cordis plugin contracts, wait-free `ArcSwap`/`moka` hot paths, stateless recurrence, and bilingual output. |
| IDs, references, and revision route are consistent | PASS | Continuous repo-wide `REQ-036..REQ-041` and `ADR-011..ADR-012` appended cleanly with zero mutation to frozen `REQ-001..REQ-035`. |
