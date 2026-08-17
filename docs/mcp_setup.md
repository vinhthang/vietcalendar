# Model Context Protocol (MCP) Setup Guide [Alpha]

> [!NOTE]
> **Feature Status: `Alpha / Testing Mode`**
> The VietCalendar MCP server is currently in alpha (`0.1.0-alpha`). It provides standard Model Context Protocol JSON-RPC 2.0 communication over standard input/output (`stdio`).

---

## 🛠️ Available MCP Tools

| Tool Name | Description | Key Arguments |
| :--- | :--- | :--- |
| **`get_today_lunar`** | Returns today's solar & lunar date in Vietnam (UTC+7), leap month status, and holiday status. | *None* |
| **`convert_solar_to_lunar`** | Converts a solar date to Vietnamese lunar date. | `date` (ISO `YYYY-MM-DD`) or `day`, `month`, `year`, `timezone` |
| **`convert_lunar_to_solar`** | Converts a lunar date to solar date. | `day`, `month`, `year`, `is_leap_month` (boolean), `timezone` |
| **`check_vietnam_holiday`** | Checks if a date is a Vietnam holiday or weekend compensatory leave day. | `date` (ISO `YYYY-MM-DD`) or `day`, `month`, `year`, `timezone` |
| **`get_year_holidays`** | Returns full schedule of Vietnam national holidays and compensatory leave for a given year. | `year` (integer, e.g. `2026`), `timezone` |

---

## 📚 Available MCP Resources

* **`calendar://today`**: Real-time JSON snapshot of today's solar/lunar dates and holiday status.
* **`calendar://holidays/{year}`**: Complete JSON holiday schedule for any requested year (e.g. `calendar://holidays/2026`).

---

## ⚙️ Client Configuration Presets

### 1. Claude Desktop

Add to your `claude_desktop_config.json`:

* **macOS:** `~/Library/Application Support/Claude/claude_desktop_config.json`
* **Windows:** `%APPDATA%\Claude\claude_desktop_config.json`

```json
{
  "mcpServers": {
    "vietcalendar": {
      "command": "/path/to/vietcalendar/target/release/vietcalendar-mcp"
    }
  }
}
```

Or using Docker:

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

---

### 2. Antigravity / Gemini CLI

Add to your `mcp.json` or Antigravity MCP settings:

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

---

### 3. Cursor IDE

In **Cursor Settings $\rightarrow$ Features $\rightarrow$ MCP $\rightarrow$ Add New MCP Server**:

* **Name:** `vietcalendar`
* **Type:** `command`
* **Command:** `cargo run --bin vietcalendar-mcp` (or path to compiled binary)

---

### 4. Manual Stdio Testing

You can interact with the server directly from the command line:

```bash
# Handshake
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | cargo run --bin vietcalendar-mcp

# Tool list
echo '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' | cargo run --bin vietcalendar-mcp

# Call conversion tool
echo '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"convert_solar_to_lunar","arguments":{"date":"2024-02-10"}}}' | cargo run --bin vietcalendar-mcp
```
