use chrono::{Datelike, Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::services::{is_vietnam_holiday, to_lunar, to_solar};

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    pub method: String,
    #[serde(default)]
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl JsonRpcResponse {
    pub fn success(id: Option<serde_json::Value>, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: Option<serde_json::Value>, code: i64, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.into(),
                data: None,
            }),
        }
    }
}

pub fn handle_request(req: JsonRpcRequest) -> Option<JsonRpcResponse> {
    let id = req.id.clone();
    match req.method.as_str() {
        "initialize" => {
            let result = json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {},
                    "resources": {}
                },
                "serverInfo": {
                    "name": "vietcalendar-mcp",
                    "version": "0.1.0-alpha (experimental)"
                }
            });
            Some(JsonRpcResponse::success(id, result))
        }
        "notifications/initialized" => {
            // Notification: No response
            None
        }
        "ping" => Some(JsonRpcResponse::success(id, json!({}))),
        "tools/list" => {
            let tools = json!({
                "tools": [
                    {
                        "name": "get_today_lunar",
                        "description": "Returns current date in Vietnam (UTC+7) in both Gregorian solar and Vietnamese lunar formats, including leap month status.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {},
                            "additionalProperties": false
                        }
                    },
                    {
                        "name": "convert_solar_to_lunar",
                        "description": "Convert a Gregorian/Solar date to a Vietnamese Lunar date.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "date": {
                                    "type": "string",
                                    "description": "Solar date in ISO format YYYY-MM-DD (e.g. '2024-02-10')"
                                },
                                "day": { "type": "integer", "description": "Solar day (1-31)" },
                                "month": { "type": "integer", "description": "Solar month (1-12)" },
                                "year": { "type": "integer", "description": "Solar year (e.g. 2024)" },
                                "timezone": { "type": "number", "description": "Timezone offset in hours (default: 7.0 for Vietnam)", "default": 7.0 }
                            }
                        }
                    },
                    {
                        "name": "convert_lunar_to_solar",
                        "description": "Convert a Vietnamese Lunar date to a Gregorian/Solar date.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "day": { "type": "integer", "description": "Lunar day (1-30)" },
                                "month": { "type": "integer", "description": "Lunar month (1-12)" },
                                "year": { "type": "integer", "description": "Lunar year (e.g. 2024)" },
                                "is_leap_month": { "type": "boolean", "description": "Set to true if converting a date within a leap month (tháng nhuận)", "default": false },
                                "timezone": { "type": "number", "description": "Timezone offset in hours (default: 7.0)", "default": 7.0 }
                            },
                            "required": ["day", "month", "year"]
                        }
                    },
                    {
                        "name": "check_vietnam_holiday",
                        "description": "Check if a date is an official Vietnam public holiday or weekend compensatory leave day (nghỉ bù).",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "date": {
                                    "type": "string",
                                    "description": "Solar date in ISO format YYYY-MM-DD (e.g. '2024-04-30')"
                                },
                                "day": { "type": "integer", "description": "Solar day (1-31)" },
                                "month": { "type": "integer", "description": "Solar month (1-12)" },
                                "year": { "type": "integer", "description": "Solar year (e.g. 2024)" },
                                "timezone": { "type": "number", "description": "Timezone offset in hours (default: 7.0)", "default": 7.0 }
                            }
                        }
                    },
                    {
                        "name": "get_year_holidays",
                        "description": "Get the complete schedule of all Vietnam public holidays and compensatory leave dates for a given solar year.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "year": { "type": "integer", "description": "Solar year (e.g. 2026)" },
                                "timezone": { "type": "number", "description": "Timezone offset in hours (default: 7.0)", "default": 7.0 }
                            },
                            "required": ["year"]
                        }
                    }
                ]
            });
            Some(JsonRpcResponse::success(id, tools))
        }
        "tools/call" => {
            let params = match req.params {
                Some(p) => p,
                None => return Some(JsonRpcResponse::error(id, -32602, "Missing params")),
            };
            let name = match params.get("name").and_then(|v| v.as_str()) {
                Some(n) => n,
                None => return Some(JsonRpcResponse::error(id, -32602, "Missing tool name")),
            };
            let args = params.get("arguments").cloned().unwrap_or(json!({}));

            let result = match name {
                "get_today_lunar" => {
                    let now_vietnam = (Utc::now() + Duration::hours(7)).date_naive();
                    let lunar = to_lunar(now_vietnam, 7.0);
                    let is_holiday = is_vietnam_holiday(now_vietnam, 7.0);
                    json!({
                        "solar_date": now_vietnam.to_string(),
                        "lunar_date": {
                            "day": lunar.day,
                            "month": lunar.month,
                            "year": lunar.year,
                            "is_leap": lunar.is_leap
                        },
                        "is_vietnam_holiday": is_holiday,
                        "timezone": "UTC+7 (Indochina Time)"
                    })
                }
                "convert_solar_to_lunar" => {
                    let (solar_date, tz) = match parse_solar_input(&args) {
                        Ok(res) => res,
                        Err(err) => return Some(JsonRpcResponse::error(id, -32602, err)),
                    };
                    let lunar = to_lunar(solar_date, tz);
                    json!({
                        "solar_date": solar_date.to_string(),
                        "lunar_date": {
                            "day": lunar.day,
                            "month": lunar.month,
                            "year": lunar.year,
                            "is_leap": lunar.is_leap
                        }
                    })
                }
                "convert_lunar_to_solar" => {
                    let day = match args.get("day").and_then(|v| v.as_i64()) {
                        Some(d) => d as i32,
                        None => return Some(JsonRpcResponse::error(id, -32602, "Missing required 'day' parameter")),
                    };
                    let month = match args.get("month").and_then(|v| v.as_i64()) {
                        Some(m) => m as i32,
                        None => return Some(JsonRpcResponse::error(id, -32602, "Missing required 'month' parameter")),
                    };
                    let year = match args.get("year").and_then(|v| v.as_i64()) {
                        Some(y) => y as i32,
                        None => return Some(JsonRpcResponse::error(id, -32602, "Missing required 'year' parameter")),
                    };
                    let is_leap = args.get("is_leap_month").and_then(|v| v.as_bool()).unwrap_or(false);
                    let tz = args.get("timezone").and_then(|v| v.as_f64()).unwrap_or(7.0);

                    match to_solar(day, month, year, is_leap, tz) {
                        Some(solar) => json!({
                            "lunar_date": {
                                "day": day,
                                "month": month,
                                "year": year,
                                "is_leap": is_leap
                            },
                            "solar_date": solar.to_string(),
                            "day": solar.day(),
                            "month": solar.month(),
                            "year": solar.year()
                        }),
                        None => return Some(JsonRpcResponse::error(id, -32602, format!("Invalid lunar date or leap month: {}/{}/{} (leap: {})", day, month, year, is_leap))),
                    }
                }
                "check_vietnam_holiday" => {
                    let (solar_date, tz) = match parse_solar_input(&args) {
                        Ok(res) => res,
                        Err(err) => return Some(JsonRpcResponse::error(id, -32602, err)),
                    };
                    let is_holiday = is_vietnam_holiday(solar_date, tz);
                    json!({
                        "date": solar_date.to_string(),
                        "weekday": solar_date.weekday().to_string(),
                        "is_holiday": is_holiday
                    })
                }
                "get_year_holidays" => {
                    let year = match args.get("year").and_then(|v| v.as_i64()) {
                        Some(y) => y as i32,
                        None => return Some(JsonRpcResponse::error(id, -32602, "Missing required 'year' parameter")),
                    };
                    let tz = args.get("timezone").and_then(|v| v.as_f64()).unwrap_or(7.0);
                    let holidays = compute_holiday_list_for_year(year, tz);
                    json!({
                        "year": year,
                        "total_holidays": holidays.len(),
                        "holidays": holidays
                    })
                }
                _ => return Some(JsonRpcResponse::error(id, -32601, format!("Tool not found: {}", name))),
            };

            let response_payload = json!({
                "content": [
                    {
                        "type": "text",
                        "text": serde_json::to_string_pretty(&result).unwrap_or_default()
                    }
                ]
            });
            Some(JsonRpcResponse::success(id, response_payload))
        }
        "resources/list" => {
            let resources = json!({
                "resources": [
                    {
                        "uri": "calendar://today",
                        "name": "Today's Vietnam Calendar",
                        "description": "Real-time snapshot of today's solar and lunar dates in Vietnam (UTC+7).",
                        "mimeType": "application/json"
                    },
                    {
                        "uri": "calendar://holidays/2026",
                        "name": "Vietnam Holidays 2026",
                        "description": "Full schedule of Vietnam national holidays and compensatory days for 2026.",
                        "mimeType": "application/json"
                    }
                ]
            });
            Some(JsonRpcResponse::success(id, resources))
        }
        "resources/read" => {
            let params = match req.params {
                Some(p) => p,
                None => return Some(JsonRpcResponse::error(id, -32602, "Missing params")),
            };
            let uri = match params.get("uri").and_then(|v| v.as_str()) {
                Some(u) => u,
                None => return Some(JsonRpcResponse::error(id, -32602, "Missing 'uri'")),
            };

            if uri == "calendar://today" {
                let now_vietnam = (Utc::now() + Duration::hours(7)).date_naive();
                let lunar = to_lunar(now_vietnam, 7.0);
                let content = json!({
                    "solar": now_vietnam.to_string(),
                    "lunar": {
                        "day": lunar.day,
                        "month": lunar.month,
                        "year": lunar.year,
                        "is_leap": lunar.is_leap
                    },
                    "is_holiday": is_vietnam_holiday(now_vietnam, 7.0)
                });
                let payload = json!({
                    "contents": [
                        {
                            "uri": uri,
                            "mimeType": "application/json",
                            "text": serde_json::to_string_pretty(&content).unwrap_or_default()
                        }
                    ]
                });
                Some(JsonRpcResponse::success(id, payload))
            } else if let Some(year_str) = uri.strip_prefix("calendar://holidays/") {
                if let Ok(year) = year_str.parse::<i32>() {
                    let holidays = compute_holiday_list_for_year(year, 7.0);
                    let payload = json!({
                        "contents": [
                            {
                                "uri": uri,
                                "mimeType": "application/json",
                                "text": serde_json::to_string_pretty(&holidays).unwrap_or_default()
                            }
                        ]
                    });
                    Some(JsonRpcResponse::success(id, payload))
                } else {
                    Some(JsonRpcResponse::error(id, -32602, format!("Invalid year in URI: {}", uri)))
                }
            } else {
                Some(JsonRpcResponse::error(id, -32602, format!("Resource not found: {}", uri)))
            }
        }
        _ => Some(JsonRpcResponse::error(id, -32601, format!("Method not found: {}", req.method))),
    }
}

fn parse_solar_input(args: &serde_json::Value) -> Result<(NaiveDate, f64), String> {
    let tz = args.get("timezone").and_then(|v| v.as_f64()).unwrap_or(7.0);
    if let Some(date_str) = args.get("date").and_then(|v| v.as_str()) {
        let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
            .map_err(|_| format!("Invalid ISO date format: '{}'. Expected YYYY-MM-DD", date_str))?;
        return Ok((date, tz));
    }
    let day = args.get("day").and_then(|v| v.as_i64()).ok_or("Missing 'date' or 'day'")? as u32;
    let month = args.get("month").and_then(|v| v.as_i64()).ok_or("Missing 'month'")? as u32;
    let year = args.get("year").and_then(|v| v.as_i64()).ok_or("Missing 'year'")? as i32;

    let date = NaiveDate::from_ymd_opt(year, month, day)
        .ok_or_else(|| format!("Invalid date: {:02}/{:02}/{}", day, month, year))?;
    Ok((date, tz))
}

#[derive(Serialize)]
pub struct HolidayInfo {
    pub date: String,
    pub name: String,
    pub category: String,
}

fn compute_holiday_list_for_year(solar_year: i32, time_zone: f64) -> Vec<HolidayInfo> {
    let mut list = Vec::new();

    // 1. Fixed Solar Holidays
    if let Some(d) = NaiveDate::from_ymd_opt(solar_year, 1, 1) {
        list.push(HolidayInfo { date: d.to_string(), name: "Tết Dương Lịch (New Year's Day)".into(), category: "FixedSolar".into() });
    }
    if let Some(d) = NaiveDate::from_ymd_opt(solar_year, 4, 30) {
        list.push(HolidayInfo { date: d.to_string(), name: "Ngày Giải Phóng Miền Nam (Reunification Day)".into(), category: "FixedSolar".into() });
    }
    if let Some(d) = NaiveDate::from_ymd_opt(solar_year, 5, 1) {
        list.push(HolidayInfo { date: d.to_string(), name: "Ngày Quốc Tế Lao Động (International Workers' Day)".into(), category: "FixedSolar".into() });
    }
    if let Some(d) = NaiveDate::from_ymd_opt(solar_year, 9, 2) {
        list.push(HolidayInfo { date: d.to_string(), name: "Ngày Quốc Khánh (National Day)".into(), category: "FixedSolar".into() });
    }

    // 2. Lunar Holidays
    for ly in [solar_year - 1, solar_year, solar_year + 1] {
        // Hung Kings: 10/3 Lunar
        if let Some(hk) = to_solar(10, 3, ly, false, time_zone) {
            if hk.year() == solar_year {
                list.push(HolidayInfo { date: hk.to_string(), name: "Giỗ Tổ Hùng Vương (Hung Kings' Commemoration)".into(), category: "Lunar".into() });
            }
        }
        // Tet: Eve, 1/1, 2/1, 3/1 Lunar
        if let Some(tet1) = to_solar(1, 1, ly, false, time_zone) {
            let eve = tet1 - Duration::days(1);
            if eve.year() == solar_year {
                list.push(HolidayInfo { date: eve.to_string(), name: "Tết Nguyên Đán (Eve - Tất Niên)".into(), category: "Lunar".into() });
            }
            if tet1.year() == solar_year {
                list.push(HolidayInfo { date: tet1.to_string(), name: "Tết Nguyên Đán (Mùng 1)".into(), category: "Lunar".into() });
            }
            if let Some(tet2) = to_solar(2, 1, ly, false, time_zone) {
                if tet2.year() == solar_year {
                    list.push(HolidayInfo { date: tet2.to_string(), name: "Tết Nguyên Đán (Mùng 2)".into(), category: "Lunar".into() });
                }
            }
            if let Some(tet3) = to_solar(3, 1, ly, false, time_zone) {
                if tet3.year() == solar_year {
                    list.push(HolidayInfo { date: tet3.to_string(), name: "Tết Nguyên Đán (Mùng 3)".into(), category: "Lunar".into() });
                }
            }
        }
    }

    list.sort_by(|a, b| a.date.cmp(&b.date));
    list.dedup_by(|a, b| a.date == b.date);
    list
}

/// Run stdio JSON-RPC loop for Model Context Protocol
pub async fn run_stdio_server() -> io::Result<()> {
    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin).lines();
    let mut stdout = io::stdout();

    while let Some(line) = reader.next_line().await? {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Ok(req) = serde_json::from_str::<JsonRpcRequest>(trimmed) {
            if let Some(resp) = handle_request(req) {
                let json_output = serde_json::to_string(&resp).unwrap_or_default();
                stdout.write_all(json_output.as_bytes()).await?;
                stdout.write_all(b"\n").await?;
                stdout.flush().await?;
            }
        } else {
            let err_resp = JsonRpcResponse::error(None, -32700, "Parse error: Invalid JSON");
            let json_output = serde_json::to_string(&err_resp).unwrap_or_default();
            stdout.write_all(json_output.as_bytes()).await?;
            stdout.write_all(b"\n").await?;
            stdout.flush().await?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_initialize() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "initialize".to_string(),
            params: None,
        };
        let resp = handle_request(req).unwrap();
        assert_eq!(resp.id, Some(json!(1)));
        let result = resp.result.unwrap();
        assert_eq!(result["serverInfo"]["name"], "vietcalendar-mcp");
        assert!(result["serverInfo"]["version"].as_str().unwrap().contains("alpha"));
    }

    #[test]
    fn test_mcp_tools_list() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(2)),
            method: "tools/list".to_string(),
            params: None,
        };
        let resp = handle_request(req).unwrap();
        let result = resp.result.unwrap();
        let tools = result["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 5);
        let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert!(names.contains(&"get_today_lunar"));
        assert!(names.contains(&"convert_solar_to_lunar"));
        assert!(names.contains(&"convert_lunar_to_solar"));
        assert!(names.contains(&"check_vietnam_holiday"));
        assert!(names.contains(&"get_year_holidays"));
    }

    #[test]
    fn test_mcp_convert_solar_to_lunar_call() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(3)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "convert_solar_to_lunar",
                "arguments": {
                    "date": "2024-02-10"
                }
            })),
        };
        let resp = handle_request(req).unwrap();
        let result = resp.result.unwrap();
        let content_text = result["content"][0]["text"].as_str().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(content_text).unwrap();
        assert_eq!(parsed["lunar_date"]["day"], 1);
        assert_eq!(parsed["lunar_date"]["month"], 1);
        assert_eq!(parsed["lunar_date"]["year"], 2024);
    }

    #[test]
    fn test_mcp_convert_lunar_to_solar_call() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(4)),
            method: "tools/call".to_string(),
            params: Some(json!({
                "name": "convert_lunar_to_solar",
                "arguments": {
                    "day": 1,
                    "month": 1,
                    "year": 2024,
                    "is_leap_month": false
                }
            })),
        };
        let resp = handle_request(req).unwrap();
        let result = resp.result.unwrap();
        let content_text = result["content"][0]["text"].as_str().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(content_text).unwrap();
        assert_eq!(parsed["solar_date"], "2024-02-10");
    }

    #[test]
    fn test_mcp_resources() {
        let req_list = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(5)),
            method: "resources/list".to_string(),
            params: None,
        };
        let resp_list = handle_request(req_list).unwrap();
        let res_list = resp_list.result.unwrap();
        assert!(res_list["resources"].as_array().unwrap().len() >= 2);

        let req_read = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(6)),
            method: "resources/read".to_string(),
            params: Some(json!({
                "uri": "calendar://holidays/2024"
            })),
        };
        let resp_read = handle_request(req_read).unwrap();
        let res_read = resp_read.result.unwrap();
        assert!(!res_read["contents"][0]["text"].as_str().unwrap().is_empty());
    }
}
