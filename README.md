# VietCalendar (Rust)

Welcome to the Rust port of the **VietCalendar** API! 🎉 

This project was successfully migrated from its original Java 11 / Maven Vert.x codebase to a high-performance, containerized Rust web application built on top of **Axum** and **Tokio**.

---

## 🚀 API Endpoints

The server exposes the following RESTful endpoints (default port: `8080` or configurable via `$PORT` / `$HTTP_PORT`):

### 1. Today's Lunar Date
* **Endpoint:** `GET /`
* **Description:** Returns the current lunar date anchored to Vietnam Indochina Time (UTC+7).
* **Response (`200 OK`):**
  ```json
  {
    "dd": 17,
    "mm": 7,
    "yyyy": 2026
  }
  ```

---

### 2. Convert Solar to Lunar (Query Parameters)
* **Endpoint:** `GET /lunar`
* **Query Parameters:**
  | Parameter | Type | Required | Default | Description |
  | :--- | :---: | :---: | :---: | :--- |
  | `dd` | integer | Yes | - | Solar day (1–31) |
  | `mm` / `MM` | integer | Yes | - | Solar month (1–12) |
  | `yyyy` | integer | Yes | - | Solar year |
  | `timezone` / `timeZone` | float | No | `7.0` | Timezone offset (default: Vietnam UTC+7) |
* **Example Request:**
  ```http
  GET /lunar?dd=12&mm=9&yyyy=2015
  ```
* **Response (`200 OK`):**
  ```json
  {
    "dd": 30,
    "mm": 7,
    "yyyy": 2015
  }
  ```
* **Error Response (`400 Bad Request`):**
  ```json
  {
    "error": "Invalid solar date: 31/02/2024"
  }
  ```

---

### 3. Convert Solar to Lunar (Path Parameter)
* **Endpoint:** `GET /convert/solar-to-lunar/{date}`
* **Path Parameter:**
  * `date` (ISO-8601 string): `YYYY-MM-DD` (e.g. `2015-09-12`)
* **Query Parameters:** `timezone` (optional, default: `7.0`)
* **Example Request:**
  ```http
  GET /convert/solar-to-lunar/2015-09-12
  ```
* **Response (`200 OK`):**
  ```json
  {
    "dd": 30,
    "mm": 7,
    "yyyy": 2015
  }
  ```

---

### 4. Convert Lunar to Solar (Path Parameter)
* **Endpoint:** `GET /convert/lunar-to-solar/{date}`
* **Path Parameter:**
  * `date` (format `YYYY-MM-DD`, where YYYY is lunar year, MM is lunar month, DD is lunar day): e.g. `2015-07-30`
* **Query Parameters:** 
  | Parameter | Type | Required | Default | Description |
  | :--- | :---: | :---: | :---: | :--- |
  | `leap` | boolean | No | `false` | `true` if converting a date within a leap month (tháng nhuận) |
  | `timezone` | float | No | `7.0` | Timezone offset |
* **Example Request:**
  ```http
  GET /convert/lunar-to-solar/2015-07-30
  ```
* **Response (`200 OK`):**
  ```json
  {
    "dd": 12,
    "mm": 9,
    "yyyy": 2015
  }
  ```

---

### 5. Check Vietnam Public Holiday
* **Endpoint:** `GET /vietnam-holiday`
* **Description:** Determines if a date is an official Vietnam holiday (including fixed solar holidays, Giỗ Tổ Hùng Vương, Tết Nguyên Đán Eve + Days 1–3, weekends, and weekend compensatory leave / nghỉ bù).
* **Query Parameters:** `dd`, `mm` (or `MM`), `yyyy`.
* **Example Request:**
  ```http
  GET /vietnam-holiday?dd=30&mm=4&yyyy=2024
  ```
* **Response (`200 OK`):**
  ```json
  true
  ```

---

### 6. Interactive Swagger / OpenAPI Documentation
* **Swagger UI:** `http://localhost:8080/swagger-ui`
* **OpenAPI 3.0 Spec:** `http://localhost:8080/api-docs/openapi.json`

---

## 🤖 Model Context Protocol (MCP) Server [Alpha]

VietCalendar provides full **Model Context Protocol (MCP)** support for AI assistants (Claude Desktop, Cursor, Antigravity, Gemini CLI, VS Code, and custom web agents).

### 1. Remote MCP over HTTP / Server-Sent Events (Public Internet Access)
Connect directly to the live server over SSE without running local processes:
* **SSE Endpoint:** `GET http://<host>:8080/mcp/sse` (or `https://your-domain.com/mcp/sse`)
* **Message Endpoint:** `POST http://<host>:8080/mcp/message?sessionId=<sessionId>`
* **CORS:** Enabled (`Access-Control-Allow-Origin: *`) for browser-based AI agents.

### 2. Local MCP over Stdio
For local CLI or direct process invocation in Claude Desktop / Cursor:
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

### Tools Provided to AI Agents
* `get_today_lunar`: Returns current solar & lunar date in Vietnam (UTC+7).
* `convert_solar_to_lunar`: Converts solar date to Vietnamese lunar date.
* `convert_lunar_to_solar`: Converts lunar date to solar date with leap month support.
* `check_vietnam_holiday`: Checks Vietnam holidays and weekend compensatory leave.
* `get_year_holidays`: Lists all public holidays and compensatory days for any year.

For complete setup options, see **[`docs/mcp_setup.md`](docs/mcp_setup.md)**.

---

## 🛠️ Running Locally & Testing

```bash
# Run unit & integration tests
cargo test

# Run the HTTP server locally on http://localhost:8080
cargo run --bin vietcalendar -- serve

# Run the MCP Server over stdio
cargo run --bin vietcalendar-mcp

# Build production Docker container
docker build -t vietcalendar-rs .
```

---

## 📖 Guides & Architectural Decisions

* **[`docs/deployment_guide.md`](docs/deployment_guide.md)**: Cloud Run, Fly.io, and Docker deployment guide.
* **[`docs/mcp_setup.md`](docs/mcp_setup.md)**: Model Context Protocol configuration guide for AI IDEs.
* **[`docs/ARCHITECTURE_DECISIONS.md`](docs/ARCHITECTURE_DECISIONS.md)**: Architecture Decision Records (ADR 1–8).
* **[`GEMINI.md`](GEMINI.md)**: Antigravity repository guidelines and breaking change protocols.


