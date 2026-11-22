# VietCalendar (Rust)

Welcome to **VietCalendar** (`vietcalendar-rs` 0.2.0) — a clean, non-blocking, performance-focused Vietnamese Luni-Solar calendar, holiday, and cultural intelligence server powered by a **Cordis-style plugin kernel** (`ArcSwap`, bounded `moka` caches, LIFO reversible effects, and live YAML hot-reload).

---

## 🧩 Architecture & Profiles

VietCalendar composes vertical plugins dynamically via declarative YAML profiles in [`config/cordis.yml`](config/cordis.yml) and overrides in [`config/cordis.patch.yml`](config/cordis.patch.yml):

* **`serve`** *(default)*: Telemetry + Astronomical Calendar (`moka` `Arc<LunarSpan>`) + Data-driven Holidays (`config/holidays/*.yml`) + Can Chi & 24 Solar Terms + BaZi/Tử Vi + Trạch Nhật + Stateless Lunar Recurrence + Retail Calendar (`config/retail_events.yml`) + Hot-swappable HTTP REST + Swagger/OpenAPI + Dynamic Agent Discovery (`/llms.txt`, `/.well-known/mcp.json`) + 16 MCP Tools & Bounded HTTP/SSE + Live `notify`/`SIGHUP` config reloader.
* **`mcp-stdio`**: Calendar + Holidays + All 5 Cultural/Niche Engines + MCP Core/Tools + Dynamic `initialize.instructions` + cancellation-aware `stdio` transport + Live config reloader.
* **`http-only`**: Telemetry + Calendar + Holidays + All 5 Cultural/Niche Engines + HTTP REST + Swagger/OpenAPI + Dynamic `/llms.txt` & `/.well-known/mcp.json` (`mcp_enabled: false`).

---

## 🤖 AI Agent Discovery (`llms.txt` & `/.well-known/mcp.json`)

VietCalendar publishes live, plugin-aware discovery manifests derived directly from the kernel's mounted `HttpRouterService` and optional `McpRegistryService` snapshots (zero hardcoded drift when plugins are toggled in `cordis.yml`):

* **[`GET /llms.txt`](llms.txt)**: `llmstxt.org`-formatted Markdown summary of active HTTP endpoints, MCP transports, live tools, resources, and URI templates.
* **`GET /llms-full.txt`**: Full agent Markdown reference including complete JSON `inputSchema` objects for every mounted MCP tool.
* **`GET /.well-known/mcp.json`**: Machine-readable JSON manifest reporting `mcp_enabled`, `protocol_version: "2024-11-05"`, dynamic `instructions`, transport URLs (`/mcp`, `/mcp/sse`), active tools, resources, templates, and REST routes.
* **MCP `initialize` Instructions**: `initialize` automatically emits live `result.instructions` describing UTC+7 guarantees and active tools/templates.
* **Repository Index**: Static Markdown overview at root [`llms.txt`](llms.txt).

---

## 🚀 HTTP REST API Endpoints

Default port: `8080` (configurable via `--port`, `$PORT`, `$HTTP_PORT`, or `cordis.yml` / `cordis.patch.yml`):

### 1. Core Lunar Date & Public Holiday Routes
* **Today's Lunar Date:** `GET /` — Returns today's lunar date anchored to Vietnam Indochina Time (`UTC+7`): `{"dd": 17, "mm": 7, "yyyy": 2026}`.
* **Convert Solar to Lunar (Query):** `GET /lunar?dd=12&mm=9&yyyy=2015` (supports `mm`/`MM`, optional `timezone`/`timeZone`, returns JSON `400` `{"error": "..."}` via `JsonQuery` on invalid types).
* **Convert Solar to Lunar (Path):** `GET /convert/solar-to-lunar/{date}` (`YYYY-MM-DD`, optional `?timezone=7.0`).
* **Convert Lunar to Solar (Path):** `GET /convert/lunar-to-solar/{date}` (`YYYY-MM-DD`, optional `?leap=false&timezone=7.0` with strict 29/30-day & leap-month validation).
* **Check Vietnam Public Holiday:** `GET /vietnam-holiday?dd=29&mm=4&yyyy=2024` (evaluates [`config/holidays/`](config/holidays/2026.yml) including `working_weekends` workday swaps like `2024-05-04`, `2025-04-26`, `2026-08-22`; unconfigured years fall back to `Sat`/`Sun`).

### 2. Niche Lunar Cultural & Intelligence Endpoints (Modular Cordis Plugins)

Enable or disable any domain/adapter individually in [`config/cordis.yml`](config/cordis.yml) or [`config/cordis.patch.yml`](config/cordis.patch.yml):

| Domain | Routes | Description |
| :--- | :--- | :--- |
| **Can Chi & 24 Tiết Khí** | `GET /canchi/{date}`, `GET /solar-terms/{year}` | Year/Month/Day/Hour Can Chi, *Nạp Âm*, and 24 *Tiết Khí* timestamps |
| **Tử Vi & BaZi** | `POST /astrology/chart`, `GET /astrology/bazi/{date}` | BaZi Four Pillars + Nam Phái Core Tử Vi (~50 stars, 12 palaces, *Tứ Hóa*, *Đại Hạn*) |
| **Trạch Nhật** | `GET /trach-nhat/day/{date}`, `POST /trach-nhat/search` | *Hoàng Đạo* hours/days, 12 *Trực*, 28 *Tú*, taboos, age clashes, ranked search (`<= 366` days) |
| **Stateless Recurrence** | `POST /recurrence/expand`, `POST /recurrence/ics` | Multi-year (`<= 50` yr) lunar recurrence with 29-day/leap policies & RFC 5545 `.ics` export |
| **Vietnam Retail Seasons** | `GET /retail/year/{year}`, `GET /retail/upcoming` | 14 hot-reloadable Vietnam retail peaks + monthly Mùng 1/Rằm with campaign milestones |
| **Agent Discovery** | `GET /llms.txt`, `GET /llms-full.txt`, `GET /.well-known/mcp.json` | Live plugin-aware Markdown & JSON manifests for LLM bots |

#### Quick Examples

```bash
# Can Chi breakdown + active Tiết Khí
curl -s "http://localhost:8080/canchi/2024-02-10?hour=12"

# Dual BaZi + Tử Vi Nam Phái birth chart
curl -s -X POST http://localhost:8080/astrology/chart \
  -H 'Content-Type: application/json' \
  -d '{"date":"1990-05-15","hour":10,"gender":"male","system":"both"}'

# Trạch Nhật ranked search for wedding/contract dates
curl -s -X POST http://localhost:8080/trach-nhat/search \
  -H 'Content-Type: application/json' \
  -d '{"start_date":"2024-02-10","end_date":"2024-02-29","occasion":"wedding","owner_birth_year":1990}'

# Export 29-day/leap-aware RFC 5545 .ics calendar feed
curl -s -X POST http://localhost:8080/recurrence/ics \
  -H 'Content-Type: application/json' \
  -d '{"preset":"vegetarian_four_days","start_date":"2025-01-01","end_date":"2025-12-31","reminder_minutes":720}'

# Upcoming Vietnam retail peaks with campaign prep milestones
curl -s "http://localhost:8080/retail/upcoming?from=2025-01-20&days=30"
```

### 3. Interactive Swagger / OpenAPI Documentation
* **Swagger UI:** `http://localhost:8080/swagger-ui`
* **OpenAPI 3.0 Spec:** `http://localhost:8080/api-docs/openapi.json` (dynamically reflects active HTTP plugins)

---

## 📅 Per-Year Holiday & Retail Configuration with Live Hot-Reload

Edit [`config/holidays/<year>.yml`](config/holidays/2026.yml), [`config/retail_events.yml`](config/retail_events.yml), or [`config/cordis.patch.yml`](config/cordis.patch.yml) while the server is running:
* **`notify` file watcher** debounces updates (`200ms`) and atomically validates & reconciles schedules/catalogs/plugins without dropping HTTP/MCP traffic.
* **Unix `SIGHUP`** triggers immediate reload; invalid YAML preserves the last known-good state.

---

## 🧠 Model Context Protocol (MCP `2024-11-05`) Server

### 1. Remote MCP over HTTP / SSE & Direct HTTP POST
* **Direct HTTP JSON-RPC POST:** `POST http://<host>:8080/mcp` or `POST http://<host>:8080/mcp/sse`
* **Legacy SSE Stream:** `GET http://<host>:8080/mcp/sse` + `POST http://<host>:8080/mcp/message?sessionId=<sessionId>` (bounded channels with `503` backpressure and strict `404` session check)
* **Agent Manifest:** `GET http://<host>:8080/.well-known/mcp.json` (plus `/llms.txt` and `/llms-full.txt`)
* **CORS:** Enabled (`Access-Control-Allow-Origin: *`)

### 2. Local MCP over Stdio
```json
{
  "mcpServers": {
    "vietcalendar": {
      "command": "cargo",
      "args": ["run", "--release", "--bin", "vietcalendar-mcp"]
    }
  }
}
```

### 3. Tools & Resources Provided to AI Agents (16 Tools, 2 Resources, 3 Templates)
* **Core Tools (`mcp_tools` — 5):** `get_today_lunar`, `convert_solar_to_lunar`, `convert_lunar_to_solar`, `check_vietnam_holiday`, `get_year_holidays`.
* **Can Chi & Solar Terms (`mcp_canchi` — 3):** `get_can_chi`, `get_solar_terms`, `get_active_solar_term`.
* **Tử Vi & BaZi (`mcp_astrology` — 2):** `generate_bazi_chart`, `generate_tuvi_chart`.
* **Trạch Nhật (`mcp_trach_nhat` — 2):** `analyze_auspicious_date`, `find_auspicious_dates`.
* **Recurrence (`mcp_recurrence` — 2):** `expand_lunar_recurrence`, `export_lunar_ics`.
* **Retail Seasons (`mcp_retail` — 2):** `get_retail_calendar`, `get_upcoming_retail_peaks`.
* **Resources & Templates:** `calendar://today`, `calendar://holidays/<current_year>`, plus templates `calendar://holidays/{year}`, `canchi://solar-terms/{year}`, and `retail://calendar/{year}`.

For complete client presets & tool payload examples, see **[`docs/mcp_setup.md`](docs/mcp_setup.md)**.

---

## 🛠️ Running Locally, Testing & Benchmarking

```bash
# Run all unit, integration & bench smoke tests
RUSTFLAGS="-D warnings" cargo test --all-targets

# Run Criterion microbenchmarks
cargo bench --bench calendar_bench

# Run HTTP + Remote MCP server (customizable --config-dir / --profile / --port)
cargo run --bin vietcalendar -- serve

# Run standalone MCP stdio server
cargo run --bin vietcalendar-mcp

# Build production Docker image (bundles /etc/vietcalendar/config)
docker build -t vietcalendar-rs .
```

---

## 📖 Guides & Architectural Decisions

* **[`llms.txt`](llms.txt)**: Repository-level Markdown overview for LLM coding agents.
* **[`docs/deployment_guide.md`](docs/deployment_guide.md)**: Cloud Run, Fly.io, and Docker deployment guide.
* **[`docs/mcp_setup.md`](docs/mcp_setup.md)**: Model Context Protocol setup presets & full 16-tool reference.
* **[`docs/ARCHITECTURE_DECISIONS.md`](docs/ARCHITECTURE_DECISIONS.md)**: Architecture Decision Records (ADR 1–15) & Cordis plugin architecture.
* **[`AGENTS.md`](AGENTS.md)**: Repository agent guidelines, architectural invariants, and breaking-change protocols.
* **[`docs/PROJECT_ANALYSIS.md`](docs/PROJECT_ANALYSIS.md)**: Codebase architecture analysis, test inventory, and resolved tech-debt tracking.
* **[`docs/RUST_STYLE_GUIDE.md`](docs/RUST_STYLE_GUIDE.md)**: Pragmatic idiomatic Rust coding standards.
* **[`docs/specs/INDEX.md`](docs/specs/INDEX.md)**: Five-document feature specifications (`REQ-001`–`REQ-043`, `ADR-001`–`ADR-013`).
