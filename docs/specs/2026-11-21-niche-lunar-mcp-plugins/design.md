# Design: Niche Lunar Calendar & Cultural Intelligence MCP/REST Plugins

Sources: [spec.md](./spec.md), [decisions.md](./decisions.md)

## Components

| Requirement / AC | Path and symbol | Responsibility / change |
| --- | --- | --- |
| REQ-036, REQ-041 / AC-036-1, AC-041-1 | `src/contracts/models.rs`, `src/contracts/{canchi,astrology,trach_nhat,recurrence,retail}.rs`, `src/contracts/mod.rs` | Define `BilingualLabel`, service traits, typed DTOs, and OpenAPI `ToSchema`/`IntoParams` derivations. |
| REQ-036 / AC-036-1, AC-036-2 | `src/plugins/canchi_solar_vn/{mod,canchi,solar_terms}.rs` | Implement `CanChiSolarPlugin` & `CanChiSolarService`: JDN Can Chi, Ngũ Hổ Độn, Ngũ Thử Độn, Nạp Âm, AA98 24 Tiết Khí bisection + bounded `moka` cache. |
| REQ-037 / AC-037-1, AC-037-2 | `src/plugins/astrology_vn/{mod,bazi,tuvi}.rs` | Implement `AstrologyPlugin` & `AstrologyService`: BaZi Four Pillars + Tử Vi Nam Phái Core (~50 major stars, 12 palaces, Tứ Hóa, Thái Tuế, Trường Sinh, Tuần/Triệt, Đại Hạn). |
| REQ-038 / AC-038-1, AC-038-2 | `src/plugins/trach_nhat_vn/{mod,rules}.rs` | Implement `TrachNhatPlugin` & `TrachNhatService`: 12 Hoàng Đạo/Hắc Đạo hours & day deities, 12 Trực, 28 Tú, major taboos, age checks, and `<= 366`-day search. |
| REQ-039 / AC-039-1, AC-039-2 | `src/plugins/recurrence_vn/{mod,ics}.rs` | Implement `LunarRecurrencePlugin` & `LunarRecurrenceService`: stateless `<= 50`-year expansion + folded CRLF RFC 5545 `.ics` export. |
| REQ-040 / AC-040-1, AC-040-2 | `src/plugins/retail_vn/{mod,loader}.rs`, `src/plugins/config_reloader.rs`, `config/retail_events.yml` | Implement `RetailCalendarPlugin`, `ArcSwap<RetailCatalog>`, strict `serde_yaml_ng` validator, `config/retail_events.yml` (14 events + Mùng 1/Rằm), and atomic reload hook. |
| REQ-041 / AC-041-1, AC-041-2 | `src/plugins/http_niches.rs`, `src/plugins/mcp_niches.rs`, `src/profiles/mod.rs`, `config/cordis.yml`, `tests/integration_test.rs`, `benches/calendar_bench.rs` | Implement 5 HTTP/OpenAPI + 5 MCP niche plugins, factory & profile wiring, E2E & cascade-unmount tests, and 7 Criterion microbenchmarks. |

## Interfaces and flow

- **Shared Astronomical Substrate (`canchi_solar_vn`):** Injects `Arc<dyn CalendarService>`. Computes day sexagenary index `(jd + 49).rem_euclid(60)`, Ngũ Hổ Độn months, Ngũ Thử Độn hours (`23:00+` shifts day stem forward by 1), 30-entry Nạp Âm table, and 24 Tiết Khí timestamps per year using 40-step bisection on AA98 `sun_longitude_aa98`, cached in a bounded `moka::sync::Cache<(i32, i64), Arc<Vec<SolarTermInfo>>>`.
- **Niche Engine Plugins (`astrology_vn`, `trach_nhat_vn`, `recurrence_vn`, `retail_vn`):** Each injects only required contract traits (`CalendarService`, `CanChiSolarService`) via `ctx.inject`, provides its own `Arc<dyn NicheService>`, and remains strictly isolated from kernel internals and peer plugin internals.
- **Retail Hot-Reload Flow:** `ConfigReloaderPlugin::reload_from_disk` stages and validates `config/holidays/*.yml` AND `config/retail_events.yml` before mutating runtime state, then reconciles `Kernel` profiles and swaps `HolidayService::replace_schedules` and `RetailCalendarService::replace_catalog` via wait-free `ArcSwap`.
- **Thin Transport Mapping (`http_niches.rs`, `mcp_niches.rs`):** Each niche registers one `Http<Niche>Plugin` (using `HttpRouterService::mount_routes`, `JsonQuery<T>`, `AppError`, and `#[utoipa::path]`) and one `Mcp<Niche>Plugin` (using `McpRegistryService::register_tool` / `register_resource_template` and `checked_param` bounds). Disposing any core niche plugin triggers Cordis LIFO dependency unmount of its dependent transport plugins.

## Boundaries and recovery

- **Zero-Panic & Index Safety:** Zero `.unwrap()`, `.expect()`, `panic!()`, or unchecked `slice[i]` indexing across all production `src/` modules; all table lookups go through checked `.get(...)` helpers returning `CalendarError`.
- **Domain Bounds:**
  - `canchi_solar_vn` / `astrology_vn`: `hour < 24`, `minute < 60`, finite `time_zone` in `-12.0..=14.0`, and `CalendarService` year/day validation.
  - `trach_nhat_vn` / `retail_vn`: date span strictly clamped to `1..=366` days; positive `birth_year`.
  - `recurrence_vn`: query horizon strictly `<= 50` years (`start <= end`), lunar day `1..=30`, lunar month `1..=12`, strict RFC 5545 `CRLF` and `<= 75`-octet UTF-8 folding.
  - `retail_vn/loader.rs`: `#[serde(deny_unknown_fields)]`, valid `(lunar_day, lunar_month)`, unique IDs, non-empty bilingual strings, non-negative offsets. Invalid reload files fail staging cleanly and leave prior `ArcSwap` catalog intact.

## Dependencies and unresolved design work

None.
