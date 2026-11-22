# Niche Lunar Calendar & Cultural Intelligence MCP/REST Plugins

- Feature ID: 2026-11-21-niche-lunar-mcp-plugins
- Type: new feature
- Status: In-Testing
- First entered In-Testing: 2026-11-21
- Related specifications: None

## Problem and source intent

User request (niche lunar MCP suite): build high-value deterministic Vietnamese Lunar Calendar (UTC+7) niche capabilities on top of the existing Cordis-based MCP and HTTP REST server so LLMs delegate exact lunisolar, Can Chi, solar-term, astrology, auspicious-date, recurrence, and Vietnam retail-season computations instead of hallucinating them:
1. Shared Vietnamese Can Chi + 24 Solar Terms astronomical foundation (`canchi_solar_vn`).
2. Vietnamese Tử Vi (Nam Phái Core) & BaZi (Tứ Trụ) calculation engine (`astrology_vn`).
3. Auspicious date selection / Trạch Nhật (`trach_nhat_vn`).
4. Stateless recurring lunar event resolver & RFC 5545 `.ics` exporter (`recurrence_vn`).
5. Data-driven Vietnam lunar retail & seasonal marketing calendar (`retail_vn`) with live hot-reload.

Follow-up constraints settled during planning:
- Expose all capabilities via **both** MCP tools/resources and HTTP REST/OpenAPI endpoints using decoupled thin plugins, enabled in `serve`, `mcp-stdio`, and `http-only` profiles and individually toggleable in `config/cordis.yml` / `config/cordis.patch.yml`.
- Keep recurring lunar events completely **stateless** (no database or server-side subscription state).
- Emit bilingual structured JSON using `(id, name_vi, name_en)` triples.
- Keep `calendar_vn::validate_year` (`-260_000..=260_000`) unchanged.

## Repository grounding

- Kernel & plugin lifecycle: [`src/kernel/mod.rs`](../../../src/kernel/mod.rs) (`Context`, `Plugin`, wait-free `ArcSwap` `ServiceRegistry`, LIFO `EffectScope`, `EventBus`).
- Existing domain services & models: [`src/contracts/mod.rs`](../../../src/contracts/mod.rs), [`src/contracts/calendar.rs`](../../../src/contracts/calendar.rs) (`CalendarService`), [`src/contracts/holidays.rs`](../../../src/contracts/holidays.rs) (`HolidayService`), [`src/contracts/http.rs`](../../../src/contracts/http.rs) (`HttpRouterService`), [`src/contracts/mcp.rs`](../../../src/contracts/mcp.rs) (`McpRegistryService`), [`src/contracts/models.rs`](../../../src/contracts/models.rs) (`LunarDate`, `CalendarError`).
- Existing astronomical math & calendar engine: [`src/plugins/calendar_vn/mod.rs`](../../../src/plugins/calendar_vn/mod.rs), [`src/plugins/calendar_vn/math.rs`](../../../src/plugins/calendar_vn/math.rs) (`jd_from_date`, `jd_to_date`, `sun_longitude_aa98`, Meeus AA98).
- Existing hot-reload & profile wiring: [`src/plugins/config_reloader.rs`](../../../src/plugins/config_reloader.rs) (`reload_from_disk`), [`src/profiles/mod.rs`](../../../src/profiles/mod.rs) (`build_default_factories`, `load_profile_entries`), [`config/cordis.yml`](../../../config/cordis.yml).

## Non-goals

- Multi-user authentication, persistence databases, or Google/Apple Calendar OAuth push sync (recurrence exports `.ics` and JSON statelessly).
- Solar Terms, Tides & Agri/Fishing advisor (niche #5 deferred out of scope).
- Minor regional/sect Tử Vi sub-star variants (100+ stars) beyond Nam Phái Core ~50 major stars.
- Mutating `calendar_vn::validate_year` bounds (`-260_000..=260_000`).

## Requirements

### REQ-036: Shared Vietnamese Can Chi and 24 Solar Terms Engine (`canchi_solar_vn`)

- Source intent: Deterministic foundation shared across BaZi, Tử Vi, Trạch Nhật, and direct REST/MCP consumers.
- Statement: WHEN queried for a Gregorian date, optional hour (`0..=23`), or year in UTC+7 (`+7.0` or explicit finite timezone `-12.0..=14.0`), THE `CanChiSolarService` implemented by `canchi_solar_vn` SHALL compute bilingual `(id, name_vi, name_en)` Can Chi pairs for the lunar year, lunar month (*Ngũ Hổ Độn*), JDN day cycle (`2000-01-01` = `Mậu Ngọ`), hour pillar (*Ngũ Thử Độn*, with 23:00–23:59 Hour of Tý rollover handling), 60 Hoa Giáp Nạp Âm elements, and all 24 astronomical solar terms (*Tiết Khí*) via Meeus AA98 solar-longitude root-finding backed by bounded `moka` caching.
- AC-036-1 — Success: `day_can_chi(2000-01-01)` yields `Mậu Ngọ`, `day_can_chi(1984-02-02)` yields `Bính Dần`, `2024-02-10` yields Year `Giáp Thìn` / Month `Bính Dần` / Day `Giáp Thìn`, 23:30 shifts day-hour lookup to next day's stem *Giờ Tý*, and `solar_terms_for_year(2024, 7.0)` returns all 24 ordered Tiết Khí with `Lập Xuân` on `2024-02-04` and exact timestamps on `Xuân Phân`, `Hạ Chí`, and `Đông Chí`.
- AC-036-2 — Boundary/failure: Out-of-range hour (`>= 24`), invalid timezone (`NaN`, `Infinity`, `< -12.0`, `> 14.0`), or out-of-range calendar year returns typed `Err(CalendarError)` without panicking or allocating unbounded caches.

### REQ-037: Vietnamese Tử Vi & BaZi Calculation Engine (`astrology_vn`)

- Source intent: Core Niche #1 ("Tu Vi & BaZi Calculation Engine" — 100% deterministic chart generation so LLMs only interpret stars and pillars).
- Statement: WHEN supplied a valid Gregorian/lunar birth timestamp (`year`, `month`, `day`, `hour`, optional `minute`), gender (`male` / `female`), chart system (`bazi`, `tuvi`, or `both`), and timezone, THE `AstrologyService` implemented by `astrology_vn` SHALL construct a complete BaZi Four Pillars chart (solar-term *Lập Xuân* year boundary and 12 *Tiết* month boundaries, Day Master, Hidden Stems / *Tàng Can*, Ten Gods / *Thập Thần*, Five Elements tally, and 8×10-year *Đại Vận* cycles) and a Vietnamese Tử Vi Nam Phái Core chart (~50 major stars across 12 Palaces with *Mệnh*/*Thân*, *Cục*, 14 *Chính Tinh*, *Lục Cát*, *Lục Sát*, *Lộc Tồn*, *Thiên Mã*, *Đào Hoa*, *Hồng Loan*/*Thiên Hỷ*, *Cô Thần*/*Quả Tú*, *Tứ Hóa*, 12 *Vòng Thái Tuế*, 12 *Vòng Trường Sinh*, *Tuần*/*Triệt*, leap-month day 1–15/16–30 split, and 12×10-year *Đại Hạn*) with bilingual metadata.
- AC-037-1 — Success: Birth timestamps straddling `2024-02-04` *Lập Xuân* produce Year `Quý Mão` before `Lập Xuân` and Year `Giáp Thìn` / Month `Bính Dần` after; Tử Vi maps exact *Cục* (*Thủy Nhị Cục*, *Hỏa Lục Cục*, etc.), places `tử vi` and `thiên phủ = (4 - tu_vi) mod 12`, distributes all 14 *Chính Tinh* and ~36 auxiliary stars across 12 unique palaces, and resolves leap-month split and gender-polarity *Đại Vận*/*Đại Hạn* directions deterministically.
- AC-037-2 — Boundary/failure: Invalid date/hour (`hour >= 24`, `minute >= 60`), invalid calendar conversion, or unsupported gender/system string rejects cleanly with `Err(CalendarError)` / `HTTP 400` / JSON-RPC `-32602` without panics.

### REQ-038: Auspicious Date Selection — Trạch Nhật (`trach_nhat_vn`)

- Source intent: Core Niche #2 ("Auspicious Date Selection (Trach Nhat) for Real Estate, Weddings & Construction").
- Statement: WHEN inspecting a single date or searching `[start_date, end_date]` for an occasion (`wedding`, `groundbreaking`, `contract_signing`, `moving_in`, `grand_opening`, `general`) with optional homeowner/couple birth years, THE `TrachNhatService` implemented by `trach_nhat_vn` SHALL evaluate 6 *Hoàng Đạo* vs *Hắc Đạo* hours and day deities (*Thanh Long*/*Minh Đường* etc.), 12 *Kiến Trừ Trực*, 28 *Nhị Thập Bát Tú*, major lunar taboos (*Tam Nương*, *Nguyệt Kỵ*, *Dương Công Kỵ Nhật*, *Sát Chủ*, *Thọ Tử*), age checks (*Tam Tai*, *Kim Lâu*, *Hoang Ốc*, *Xung Tuổi*), and return scored, bilingual auspicious candidates sorted by score then date.
- AC-038-1 — Success: Analyzing `2024-02-10` returns Day `Giáp Thìn`, Lunar `01/01/2024`, `Trực Mãn`, and `28 Tú` `Sao Liễu`, with exact 6 *Hoàng Đạo* hours for `Thìn`; `birth_year = 1990` (*Canh Ngọ*) flags *Tam Tai* in `Thân`/`Dậu`/`Tuất` years and evaluates *Kim Lâu*/*Hoang Ốc*; range search returns ranked results strictly ordered by `(score DESC, date ASC)`.
- AC-038-2 — Boundary/failure: Ranges with `end < start`, spans exceeding `366` days, invalid occasion or birth year (`<= 0`), or invalid date return typed domain error (`Err(CalendarError)` -> `HTTP 400` / JSON-RPC `-32602`).

### REQ-039: Stateless Recurring Lunar Event Manager & `.ics` Export (`recurrence_vn`)

- Source intent: Core Niche #3 ("Recurring Lunar Event Manager" handling 29-day and leap months cleanly without stateful DB storage).
- Statement: WHEN given a lunar recurrence rule (`monthly` or `yearly` on `(lunar_day, optional lunar_month)` or preset `NewAndFullMoon` / `VegetarianFourDays` / `VegetarianSixDays` / `VegetarianTenDays`), a Gregorian date window or year span, `Day30Policy` (`Skip` / `LastDayOfMonth`), `LeapMonthPolicy` (`PrimaryOnly` / `IncludeLeap` / `LeapOnly`), and timezone, THE `LunarRecurrenceService` implemented by `recurrence_vn` SHALL resolve exact Gregorian occurrences statelessly via `CalendarService` and optionally serialize an RFC 5545 `.ics` VCALENDAR (`CRLF` line endings, RFC 5545 line folding `<= 75` octets, optional `VALARM`).
- AC-039-1 — Success: Day-30 recurrence applies `Skip` vs `LastDayOfMonth` (yielding day 29 on 29-day *tháng thiếu*); leap-month rules handle `2023` (leap 2) and `2025` (leap 6) accurately across `PrimaryOnly`, `IncludeLeap`, and `LeapOnly`; vegetarian presets emit expected days; `.ics` export validates CRLF endings, `BEGIN:VCALENDAR`..`END:VCALENDAR`, UTF-8 octet folding, and `VALARM`.
- AC-039-2 — Boundary/failure: Query windows exceeding `50` years (`> 50` years) or `start > end`, invalid lunar day (`0` or `> 30`) / month (`0` or `> 12`), or malformed preset names reject cleanly with typed error (`Err` -> `HTTP 400` / JSON-RPC `-32602`).

### REQ-040: Data-Driven Vietnam Retail & Seasonal Marketing Calendar (`retail_vn`)

- Source intent: Core Niche #4 ("Lunar Retail & Seasonal Marketing Calendar") with hot-reload parity with `holidays_vn`.
- Statement: WHEN queried by year or `[start_date, start_date + days]` (`days <= 366`), optional retail category filter, and timezone, THE `RetailCalendarService` implemented by `retail_vn` SHALL resolve Vietnam retail & cultural peaks from `config/retail_events.yml` (backed by wait-free `ArcSwap` storage and strict YAML validation) plus algorithmic `monthly_mung_1_ram` recurrences, compute lead-time `campaign_milestones` (`prep_start`, `early_bird`, `peak`), and integrate fail-safe staging/commit into `ConfigReloaderPlugin::reload_from_disk`.
- AC-040-1 — Success: Default `config/retail_events.yml` defines 14 official Vietnam retail events plus `monthly_mung_1_ram`; `retail_events_for_year(2025)` resolves `tat_nien_giao_thua` (lunar 30/12 mapped on 29-day Chạp via `last_day_of_month`) to `2025-01-28`, `ngay_via_than_tai` (`10/01`) to `2025-02-07`, `trung_thu` (`15/08`) to `2025-10-06`, and computes exact prep/early-bird/peak dates; valid reload swaps `ArcSwap` atomically while corrupt YAML preserves prior active catalog.
- AC-040-2 — Boundary/failure: Unknown `category`, `days > 366` (or `<= 0`), or invalid `config/retail_events.yml` fields (unknown keys via `deny_unknown_fields`, invalid lunar day/month, duplicate `id`, negative lead offsets, empty bilingual names) reject cleanly without mutating live state.

### REQ-041: Thin HTTP/OpenAPI & MCP Plugin Exposure, Toggleable Profiles & Benchmarks

- Source intent: Maintain Cordis plugin modularity (`http_niches`, `mcp_niches`), zero-panic wait-free hot paths, toggleable YAML profiles, and Criterion telemetry.
- Statement: THE server SHALL provide 5 thin HTTP/OpenAPI plugins (`http_canchi`, `http_astrology`, `http_trach_nhat`, `http_recurrence`, `http_retail` exposing 10 versioned REST routes and `utoipa` schemas) and 5 thin MCP plugins (`mcp_canchi`, `mcp_astrology`, `mcp_trach_nhat`, `mcp_recurrence`, `mcp_retail` exposing 11 tools and 2 resource templates `canchi://solar-terms/{year}`, `retail://calendar/{year}`), registered in `PluginFactoryRegistry` and toggleable across `config/cordis.yml` (`serve`, `mcp-stdio`, `http-only`), with clean LIFO route/tool/OpenAPI unmount disposal, zero production `.unwrap()`/`.expect()`/`panic!()`, zero `RwLock` on request paths, and Criterion microbenchmarks.
- AC-041-1 — Success: All 10 REST endpoints return HTTP 200 JSON (and `.ics` text/calendar where requested); `/api-docs/openapi.json` merges all niche paths & schemas; MCP `tools/list` exposes 16 tools total (5 existing + 11 niche) and `resources/templates/list` exposes 3 templates; disposing an individual niche core plugin cascades unmount of its dependent HTTP/MCP plugins cleanly (`HTTP 404` and removal from OpenAPI & MCP `tools/list`); 7 new Criterion benchmarks complete offline.
- AC-041-2 — Boundary/failure: Invalid HTTP path/query/JSON parameters return `HTTP 400` `ErrorResponse`; invalid MCP params return JSON-RPC `-32602`; disabling a niche plugin in YAML removes its routes and MCP tools cleanly while keeping all other plugins functional.

## Assumptions and owner-deferred questions

- BaZi day transition default follows standard lunar midnight `23:00` (*Giờ Tý* shifting day lookup forward by 1 day for *Ngũ Thử Độn* and treating `day_hour_split` deterministically via `use_early_late_ty` or standard early-Tý rollover documented on input DTO).
- Tử Vi chart covers Nam Phái Core (~50 major stars, Triệt/Tuần, Tứ Hóa, Vòng Thái Tuế, Vòng Trường Sinh, 10-year Đại Hạn); micro stars (*Bác Sĩ 12*, *Tướng Tinh*, minor *Tạp Diệu*) are deferred as non-core unless requested later.
- No owner-deferred `[PROVISIONAL]` requirements remain open.

## Approvals and lifecycle history

- `2026-11-21`: Initial specification authored from user request ("Niche Lunar Calendar MCP Server Ideas") and settled `/grill-me` design interview choices.
- `2026-11-21`: `Planning -> In-Progress` — Gate 1 and Gate 2 passed cleanly; implementation authorized under `/goal` execution phase.
- `2026-11-21`: `In-Progress -> In-Testing` — All Tasks 1–6 completed; all 67 lib unit tests, 7 integration tests, 13 Criterion benchmarks, static safety/arch-guard checks, and targeted `cargo mutants` (`110/112` = `98.2%` kill rate) verified in [Run 1](./verification.md#run-1--2026-11-21).
