# Model Context Protocol (MCP) & Agent Discovery Setup Guide

VietCalendar (`vietcalendar-rs` `0.2.0`) provides standard **Model Context Protocol (MCP `2024-11-05`)** JSON-RPC 2.0 support across three transports powered by `McpCorePlugin` (dynamic `initialize.instructions`), `McpToolsPlugin`, `mcp_niches`, `McpStdioPlugin`, and `McpHttpPlugin`:

1. **Local Stdio (`vietcalendar-mcp` / `vietcalendar mcp`)**
2. **Remote HTTP/SSE (`GET /mcp/sse` + `POST /mcp/message?sessionId=...`)** with bounded per-session queues (`503` backpressure) and strict `404` session checks
3. **Direct Streamable HTTP JSON-RPC POST (`POST /mcp` and `POST /mcp/sse`)** with permissive CORS

When HTTP is running (`serve` or `http-only`), autonomous agents can also inspect live server capabilities at runtime via:
* **`GET /llms.txt`** (`text/markdown`): Concise `llmstxt.org` Markdown listing active endpoints, MCP URLs, tools, resources, and templates
* **`GET /llms-full.txt`** (`text/markdown`): Expanded Markdown including full JSON `inputSchema` schemas for each live tool
* **`GET /.well-known/mcp.json`** (`application/json`): Machine-readable discovery manifest reporting `mcp_enabled`, `protocol_version`, `instructions`, transports, active tools, resources, URI templates, and OpenAPI routes

---

## 🛠️ Available MCP Tools (16 Total Across 6 Plugins)

Disable any tool group cleanly in [`config/cordis.yml`](../config/cordis.yml) or [`config/cordis.patch.yml`](../config/cordis.patch.yml); `initialize.instructions`, `tools/list`, `/llms.txt`, and `/.well-known/mcp.json` update dynamically.

### Core Calendar & Holidays (`mcp_tools` — 5 Tools)

| Tool Name | Description | Key Arguments |
| :--- | :--- | :--- |
| **`get_today_lunar`** | Returns today's solar & lunar date in Vietnam (`UTC+7`), leap month status, and holiday status. | *None* |
| **`convert_solar_to_lunar`** | Converts a solar date to Vietnamese lunar date. | `date` (`YYYY-MM-DD`) or `day`, `month`, `year`, `timezone` |
| **`convert_lunar_to_solar`** | Converts a lunar date to solar date with strict 29/30-day validation. | `day`, `month`, `year`, `is_leap_month` (boolean), `timezone` |
| **`check_vietnam_holiday`** | Checks if a date is a Vietnam holiday, bridge day, or weekend (honors `working_weekends` swaps). | `date` (`YYYY-MM-DD`) or `day`, `month`, `year`, `timezone` |
| **`get_year_holidays`** | Returns the complete data-driven holiday schedule and `working_weekends` for any solar year. | `year` (integer, e.g. `2026`), `timezone` |

### Can Chi & 24 Solar Terms (`mcp_canchi` — 3 Tools)

| Tool Name | Description | Key Arguments |
| :--- | :--- | :--- |
| **`get_can_chi`** | Computes bilingual Year/Month (*Ngũ Hổ Độn* & *Tiết Khí*), JDN Day, Hour (*Ngũ Thử Độn* with `23:00` rollover), and *Nạp Âm*. | `date` (`YYYY-MM-DD`), optional `hour`, `minute`, `timezone` |
| **`get_solar_terms`** | Lists all 24 astronomical solar terms (*Tiết Khí*) in chronological local timestamps for a year. | `year` (integer), optional `timezone` |
| **`get_active_solar_term`** | Computes ecliptic solar longitude plus current and next *Tiết Khí* at an instant. | `date` (`YYYY-MM-DD`), optional `hour`, `minute`, `timezone` |

### Tử Vi & BaZi (`mcp_astrology` — 2 Tools)

| Tool Name | Description | Key Arguments |
| :--- | :--- | :--- |
| **`generate_bazi_chart`** | Generates Four Pillars (*Tiết Khí* bounds, *Tàng Can*, *Thập Thần*, *Ngũ Hành* balance, 8×10yr *Đại Vận*). | `date` (`YYYY-MM-DD`), `hour`, `gender` (`male`/`female`), optional `minute`, `is_lunar`, `is_leap_month`, `time_zone` |
| **`generate_tuvi_chart`** | Generates Vietnamese Tử Vi Nam Phái Core chart (~50 stars, 12 palaces, *Cục*, *Tứ Hóa*, *Tuần*/*Triệt*, *Đại Hạn*). | `date`, `hour`, `gender`, optional `system` (`tuvi`/`both`), `minute`, `is_lunar`, `is_leap_month`, `time_zone` |

### Trạch Nhật — Auspicious Dates (`mcp_trach_nhat` — 2 Tools)

| Tool Name | Description | Key Arguments |
| :--- | :--- | :--- |
| **`analyze_auspicious_date`** | Evaluates *Hoàng Đạo* hours/days, 12 *Trực*, 28 *Tú*, major taboos, *Tam Tai*/*Kim Lâu*/*Hoang Ốc*/*Xung Tuổi*, and score. | `date` (`YYYY-MM-DD`), optional `occasion`, `owner_birth_year`, `partner_birth_year`, `timezone` |
| **`find_auspicious_dates`** | Searches and ranks top candidate dates across `<= 366` days ordered `(score DESC, solar_date ASC)`. | `start_date`, `end_date`, `occasion`, optional `owner_birth_year`, `partner_birth_year`, `min_score`, `limit`, `time_zone` |

### Stateless Lunar Recurrence (`mcp_recurrence` — 2 Tools)

| Tool Name | Description | Key Arguments |
| :--- | :--- | :--- |
| **`expand_lunar_recurrence`** | Expands lunar recurring rules or presets across `<= 50` years handling 29-day & leap months. | `start_date`, `end_date`, `preset` or (`frequency`, `lunar_day`, `lunar_month`), `day30_policy`, `leap_month_policy`, `time_zone` |
| **`export_lunar_ics`** | Renders RFC 5545 VCALENDAR payload (`CRLF` + `<= 75`-octet UTF-8 folding + optional `VALARM`). | Same as `expand_lunar_recurrence`, plus optional `title`, `description`, `reminder_minutes` |

### Vietnam Retail Calendar (`mcp_retail` — 2 Tools)

| Tool Name | Description | Key Arguments |
| :--- | :--- | :--- |
| **`get_retail_calendar`** | Resolves 14 official Vietnam retail peaks + monthly Mùng 1/Rằm with campaign milestones. | `year` (integer), optional `category`, `timezone` |
| **`get_upcoming_retail_peaks`** | Lists upcoming retail peaks within `1..=366` days with `prep_start`, `early_bird`, and `peak` dates. | Optional `from_date` (`YYYY-MM-DD`), `days` (`1..=366`), `category`, `timezone` |

---

## 📚 Available MCP Resources & Templates

### Concrete Resources (`resources/list`)
* **`calendar://today`**: Real-time JSON snapshot of today's solar/lunar dates and holiday status in Vietnam (`UTC+7`).
* **`calendar://holidays/<current_year>`**: Concrete JSON holiday schedule for the current UTC+7 year.

### URI Templates (`resources/templates/list`)
* **`calendar://holidays/{year}`**: Holiday schedule and `working_weekends` for any requested year (e.g. `calendar://holidays/2026`).
* **`canchi://solar-terms/{year}`**: All 24 astronomical solar terms (*24 Tiết Khí*) in UTC+7 for any Gregorian year (e.g. `canchi://solar-terms/2024`).
* **`retail://calendar/{year}`**: Complete Vietnam retail peak & monthly Mùng 1/Rằm calendar with campaign milestones for any year (e.g. `retail://calendar/2025`).

---

## ⚙️ Client Configuration Presets

### 1. Claude Desktop (`stdio`)

Add to your `claude_desktop_config.json`:

* **macOS:** `~/Library/Application Support/Claude/claude_desktop_config.json`
* **Windows:** `%APPDATA%\Claude\claude_desktop_config.json`

```json
{
  "mcpServers": {
    "vietcalendar": {
      "command": "/path/to/vietcalendar/target/release/vietcalendar-mcp",
      "env": {
        "VIETCALENDAR_CONFIG_DIR": "/path/to/vietcalendar/config"
      }
    }
  }
}
```

Or using Docker (`vietcalendar mcp`):

```json
{
  "mcpServers": {
    "vietcalendar": {
      "command": "docker",
      "args": ["run", "-i", "--rm", "vietcalendar-rs", "vietcalendar", "mcp"]
    }
  }
}
```

### 2. Antigravity / Gemini CLI (`stdio` or Remote HTTP)

For local `stdio`:

```json
{
  "mcpServers": {
    "vietcalendar": {
      "command": "vietcalendar-mcp",
      "args": []
    }
  }
}
```

For remote HTTP (`vietcalendar serve`):
* **Streamable HTTP POST:** `http://localhost:8080/mcp`
* **Legacy SSE:** `http://localhost:8080/mcp/sse`
* **Agent Discovery Index:** `http://localhost:8080/llms.txt` / `http://localhost:8080/.well-known/mcp.json`

### 3. Cursor IDE

In **Cursor Settings $\rightarrow$ Features $\rightarrow$ MCP $\rightarrow$ Add New MCP Server**:

* **Stdio:** `Type: command`, `Command: cargo run --bin vietcalendar-mcp`
* **Remote SSE / HTTP:** `Type: sse`, `URL: http://localhost:8080/mcp/sse`

### 4. Manual CLI & HTTP Testing

```bash
# Stdio handshake (includes dynamic instructions) + 2026 holiday list
printf '{"jsonrpc":"2.0","id":1,"method":"initialize"}\n{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"get_year_holidays","arguments":{"year":2026}}}\n' | cargo run --quiet --bin vietcalendar-mcp

# Fetch agent discovery manifest & llms.txt from running HTTP server
curl -s http://localhost:8080/llms.txt
curl -s http://localhost:8080/.well-known/mcp.json

# Direct HTTP POST against running server (template list + Tử Vi chart)
curl -s -X POST http://localhost:8080/mcp \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"resources/templates/list"}'

curl -s -X POST http://localhost:8080/mcp \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"generate_tuvi_chart","arguments":{"date":"1990-05-15","hour":10,"gender":"male"}}}'
```
