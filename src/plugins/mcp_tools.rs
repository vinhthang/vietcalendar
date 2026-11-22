//! `McpToolsPlugin`: reversibly registers all 5 VietCalendar MCP tools and calendar resources.

use std::sync::Arc;

use chrono::{Datelike, Duration, NaiveDate, Utc};
use serde_json::{json, Value};

use crate::contracts::calendar::CalendarService;
use crate::contracts::holidays::HolidayService;
use crate::contracts::mcp::{
    McpError, McpRegistryService, McpResourceSpec, McpResourceTemplateSpec, McpToolSpec,
};
use crate::kernel::context::Context;
use crate::kernel::effect::BoxFuture;
use crate::kernel::error::KernelError;
use crate::kernel::fiber::{Plugin, PluginMeta};

fn extract_i32(args: &Value, key: &str) -> Result<i32, McpError> {
    let raw = args
        .get(key)
        .and_then(|v| v.as_i64())
        .ok_or_else(|| McpError::InvalidParams(format!("Missing required integer '{key}'")))?;
    i32::try_from(raw)
        .map_err(|_| McpError::InvalidParams(format!("Parameter '{key}' out of i32 bounds: {raw}")))
}

fn extract_u32(args: &Value, key: &str) -> Result<u32, McpError> {
    let raw = args
        .get(key)
        .and_then(|v| v.as_i64())
        .ok_or_else(|| McpError::InvalidParams(format!("Missing required integer '{key}'")))?;
    u32::try_from(raw)
        .map_err(|_| McpError::InvalidParams(format!("Parameter '{key}' out of u32 bounds: {raw}")))
}

fn extract_opt_f64(args: &Value, key: &str, default: f64) -> Result<f64, McpError> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(default),
        Some(v) => {
            let tz = v.as_f64().ok_or_else(|| {
                McpError::InvalidParams(format!("Parameter '{key}' must be a number"))
            })?;
            if !tz.is_finite() || !(-12.0..=14.0).contains(&tz) {
                return Err(McpError::InvalidParams(format!(
                    "Invalid timezone offset: {tz}"
                )));
            }
            Ok(tz)
        }
    }
}

fn extract_opt_bool(args: &Value, key: &str, default: bool) -> Result<bool, McpError> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(default),
        Some(v) => v
            .as_bool()
            .ok_or_else(|| McpError::InvalidParams(format!("Parameter '{key}' must be a boolean"))),
    }
}

fn parse_solar_input(args: &Value) -> Result<(NaiveDate, f64), McpError> {
    let tz = extract_opt_f64(args, "timezone", 7.0)?;
    if let Some(date_str) = args.get("date").and_then(|v| v.as_str()) {
        let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d").map_err(|_| {
            McpError::InvalidParams(format!(
                "Invalid ISO date format: '{date_str}'. Expected YYYY-MM-DD"
            ))
        })?;
        return Ok((date, tz));
    }
    if let Some(v) = args.get("date") {
        if !v.is_null() {
            return Err(McpError::InvalidParams(
                "Parameter 'date' must be a YYYY-MM-DD string".to_string(),
            ));
        }
    }

    let day = extract_u32(args, "day")?;
    let month = extract_u32(args, "month")?;
    let year = extract_i32(args, "year")?;
    let date = NaiveDate::from_ymd_opt(year, month, day).ok_or_else(|| {
        McpError::InvalidParams(format!("Invalid solar date: {day:02}/{month:02}/{year}"))
    })?;
    Ok((date, tz))
}

fn format_year_holidays_json(holidays: &dyn HolidayService, year: i32) -> Value {
    let sched = holidays.year_holidays(year);
    let holiday_items: Vec<Value> = sched
        .entries()
        .iter()
        .map(|entry| {
            json!({
                "date": entry.date.to_string(),
                "name": entry.name,
                "category": entry.category
            })
        })
        .collect();
    let working_weekends: Vec<String> = sched
        .working_weekends()
        .iter()
        .map(|d| d.to_string())
        .collect();

    json!({
        "year": year,
        "configured": sched.configured,
        "source": sched.source,
        "total_holidays": holiday_items.len(),
        "holidays": holiday_items,
        "working_weekends": working_weekends
    })
}

/// Cordis plugin registering all 5 VietCalendar tools and calendar resources into `McpRegistryService`.
#[derive(Debug, Default)]
pub struct McpToolsPlugin;

impl Plugin for McpToolsPlugin {
    fn meta(&self) -> PluginMeta {
        PluginMeta::new("mcp_tools")
            .requires::<dyn CalendarService>()
            .requires::<dyn HolidayService>()
            .requires::<dyn McpRegistryService>()
    }

    fn mount<'a>(&'a self, ctx: Context, _config: Value) -> BoxFuture<'a, Result<(), KernelError>> {
        Box::pin(async move {
            let cal = ctx.inject::<dyn CalendarService>()?;
            let holidays = ctx.inject::<dyn HolidayService>()?;
            let registry = ctx.inject::<dyn McpRegistryService>()?;

            // Tool 1: get_today_lunar
            {
                let cal_c = Arc::clone(&cal);
                let hol_c = Arc::clone(&holidays);
                let disp = registry.register_tool(McpToolSpec {
                    name: "get_today_lunar".into(),
                    description: "Returns current date in Vietnam (UTC+7) in both Gregorian solar and Vietnamese lunar formats, including leap month and holiday status.".into(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {},
                        "additionalProperties": false
                    }),
                    handler: Arc::new(move |_| {
                        let now_vn = (Utc::now() + Duration::hours(7)).date_naive();
                        let lunar = cal_c
                            .to_lunar(now_vn, 7.0)
                            .map_err(|e| McpError::Internal(e.to_string()))?;
                        let is_holiday = hol_c.is_holiday(now_vn);
                        Ok(json!({
                            "solar_date": now_vn.to_string(),
                            "lunar_date": lunar,
                            "is_vietnam_holiday": is_holiday,
                            "timezone": "UTC+7 (Indochina Time)"
                        }))
                    }),
                });
                ctx.on_dispose_sync(disp);
            }

            // Tool 2: convert_solar_to_lunar
            {
                let cal_c = Arc::clone(&cal);
                let disp = registry.register_tool(McpToolSpec {
                    name: "convert_solar_to_lunar".into(),
                    description: "Convert a Gregorian/Solar date to a Vietnamese Lunar date.".into(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "date": { "type": "string", "description": "Solar date in ISO format YYYY-MM-DD (e.g. '2024-02-10')" },
                            "day": { "type": "integer", "description": "Solar day (1-31)" },
                            "month": { "type": "integer", "description": "Solar month (1-12)" },
                            "year": { "type": "integer", "description": "Solar year (e.g. 2024)" },
                            "timezone": { "type": "number", "description": "Timezone offset in hours (default: 7.0)", "default": 7.0 }
                        }
                    }),
                    handler: Arc::new(move |args| {
                        let (solar_date, tz) = parse_solar_input(&args)?;
                        let lunar = cal_c
                            .to_lunar(solar_date, tz)
                            .map_err(|e| McpError::InvalidParams(e.to_string()))?;
                        Ok(json!({
                            "solar_date": solar_date.to_string(),
                            "lunar_date": lunar
                        }))
                    }),
                });
                ctx.on_dispose_sync(disp);
            }

            // Tool 3: convert_lunar_to_solar
            {
                let cal_c = Arc::clone(&cal);
                let disp = registry.register_tool(McpToolSpec {
                    name: "convert_lunar_to_solar".into(),
                    description: "Convert a Vietnamese Lunar date to a Gregorian/Solar date with strict 29/30-day validation.".into(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "day": { "type": "integer", "description": "Lunar day (1-30)" },
                            "month": { "type": "integer", "description": "Lunar month (1-12)" },
                            "year": { "type": "integer", "description": "Lunar year (e.g. 2024)" },
                            "is_leap_month": { "type": "boolean", "description": "True when converting a date in a leap month", "default": false },
                            "timezone": { "type": "number", "description": "Timezone offset in hours (default: 7.0)", "default": 7.0 }
                        },
                        "required": ["day", "month", "year"]
                    }),
                    handler: Arc::new(move |args| {
                        let day = extract_i32(&args, "day")?;
                        let month = extract_i32(&args, "month")?;
                        let year = extract_i32(&args, "year")?;
                        let is_leap = extract_opt_bool(&args, "is_leap_month", false)?;
                        let tz = extract_opt_f64(&args, "timezone", 7.0)?;

                        let solar = cal_c
                            .to_solar(day, month, year, is_leap, tz)
                            .map_err(|e| McpError::InvalidParams(e.to_string()))?;

                        Ok(json!({
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
                        }))
                    }),
                });
                ctx.on_dispose_sync(disp);
            }

            // Tool 4: check_vietnam_holiday
            {
                let hol_c = Arc::clone(&holidays);
                let disp = registry.register_tool(McpToolSpec {
                    name: "check_vietnam_holiday".into(),
                    description: "Check if a date is a Vietnam holiday, bridge day, or weekend under data-driven per-year schedules (honors working_weekends overrides).".into(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "date": { "type": "string", "description": "Solar date in ISO format YYYY-MM-DD (e.g. '2024-04-30')" },
                            "day": { "type": "integer", "description": "Solar day (1-31)" },
                            "month": { "type": "integer", "description": "Solar month (1-12)" },
                            "year": { "type": "integer", "description": "Solar year (e.g. 2024)" },
                            "timezone": { "type": "number", "description": "Timezone offset in hours (default: 7.0)", "default": 7.0 }
                        }
                    }),
                    handler: Arc::new(move |args| {
                        let (solar_date, tz) = parse_solar_input(&args)?;
                        let is_holiday = hol_c
                            .is_holiday_with_timezone(solar_date, tz)
                            .map_err(|e| McpError::InvalidParams(e.to_string()))?;
                        Ok(json!({
                            "date": solar_date.to_string(),
                            "weekday": solar_date.weekday().to_string(),
                            "is_holiday": is_holiday
                        }))
                    }),
                });
                ctx.on_dispose_sync(disp);
            }

            // Tool 5: get_year_holidays
            {
                let hol_c = Arc::clone(&holidays);
                let disp = registry.register_tool(McpToolSpec {
                    name: "get_year_holidays".into(),
                    description: "Get the complete configured Vietnam holiday schedule (including compensatory, bridge, and working_weekends overrides) for a solar year.".into(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "year": { "type": "integer", "description": "Solar year (e.g. 2026)" },
                            "timezone": { "type": "number", "description": "Timezone offset in hours (default: 7.0)", "default": 7.0 }
                        },
                        "required": ["year"]
                    }),
                    handler: Arc::new(move |args| {
                        let year = extract_i32(&args, "year")?;
                        let _tz = extract_opt_f64(&args, "timezone", 7.0)?;
                        Ok(format_year_holidays_json(hol_c.as_ref(), year))
                    }),
                });
                ctx.on_dispose_sync(disp);
            }

            // Resource 1: calendar://today
            {
                let cal_c = Arc::clone(&cal);
                let hol_c = Arc::clone(&holidays);
                let disp = registry.register_resource(McpResourceSpec {
                    uri: "calendar://today".into(),
                    name: "Today's Vietnam Calendar".into(),
                    description:
                        "Real-time snapshot of today's solar and lunar dates in Vietnam (UTC+7)."
                            .into(),
                    mime_type: "application/json".into(),
                    handler: Arc::new(move |_| {
                        let now_vn = (Utc::now() + Duration::hours(7)).date_naive();
                        let lunar = cal_c
                            .to_lunar(now_vn, 7.0)
                            .map_err(|e| McpError::Internal(e.to_string()))?;
                        Ok(json!({
                            "solar": now_vn.to_string(),
                            "lunar": lunar,
                            "is_holiday": hol_c.is_holiday(now_vn)
                        }))
                    }),
                });
                ctx.on_dispose_sync(disp);
            }

            // Resource 2: Dynamic current-year concrete resource calendar://holidays/<current_year>
            {
                let current_year = (Utc::now() + Duration::hours(7)).date_naive().year();
                let hol_c = Arc::clone(&holidays);
                let disp = registry.register_resource(McpResourceSpec {
                    uri: format!("calendar://holidays/{current_year}"),
                    name: format!("Vietnam Holidays {current_year}"),
                    description: format!("Vietnam national holiday schedule for {current_year}."),
                    mime_type: "application/json".into(),
                    handler: Arc::new(move |_| {
                        Ok(format_year_holidays_json(hol_c.as_ref(), current_year))
                    }),
                });
                ctx.on_dispose_sync(disp);
            }

            // Resource Template: calendar://holidays/{year}
            {
                let hol_c = Arc::clone(&holidays);
                let disp = registry.register_resource_template(McpResourceTemplateSpec {
                    uri_template: "calendar://holidays/{year}".into(),
                    name: "Vietnam Holidays by Year".into(),
                    description: "Data-driven Vietnam national holidays and working-weekend swaps for any requested solar year.".into(),
                    mime_type: "application/json".into(),
                    prefix: "calendar://holidays/".into(),
                    handler: Arc::new(move |suffix| {
                        let year = suffix.parse::<i32>().map_err(|_| {
                            McpError::InvalidParams(format!(
                                "Invalid year in URI: calendar://holidays/{suffix}"
                            ))
                        })?;
                        Ok(format_year_holidays_json(hol_c.as_ref(), year))
                    }),
                });
                ctx.on_dispose_sync(disp);
            }

            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::mcp::{JsonRpcRequest, McpRegistryService};
    use crate::kernel::Kernel;
    use crate::plugins::calendar_vn::VietnamCalendarPlugin;
    use crate::plugins::holidays_vn::VietnamHolidaysPlugin;
    use crate::plugins::mcp_core::McpCorePlugin;

    async fn mount_mcp_kernel() -> (Kernel, Arc<dyn McpRegistryService>) {
        let kernel = Kernel::new();
        kernel
            .mount("cal", Arc::new(VietnamCalendarPlugin), json!({}))
            .await
            .unwrap();
        kernel
            .mount(
                "hol",
                Arc::new(VietnamHolidaysPlugin),
                json!({ "config_base_dir": "config", "holidays_dir": "holidays" }),
            )
            .await
            .unwrap();
        kernel
            .mount("mcp_core", Arc::new(McpCorePlugin), json!({}))
            .await
            .unwrap();
        kernel
            .mount("mcp_tools", Arc::new(McpToolsPlugin), json!({}))
            .await
            .unwrap();
        let reg = kernel.inject::<dyn McpRegistryService>().unwrap();
        (kernel, reg)
    }

    fn call_tool(
        reg: &dyn McpRegistryService,
        name: &str,
        arguments: Value,
    ) -> (Option<Value>, Option<i64>) {
        let resp = reg
            .dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(1)),
                method: "tools/call".into(),
                params: Some(json!({ "name": name, "arguments": arguments })),
            })
            .unwrap();
        (resp.result, resp.error.map(|e| e.code))
    }

    #[tokio::test]
    async fn test_mcp_tools_rejects_malformed_optional_parameter_types() {
        let (kernel, reg) = mount_mcp_kernel().await;

        let (_, err1) = call_tool(
            reg.as_ref(),
            "convert_solar_to_lunar",
            json!({ "date": "2024-02-10", "timezone": "bad" }),
        );
        assert_eq!(err1, Some(-32602), "timezone string must return -32602");

        let (_, err2) = call_tool(
            reg.as_ref(),
            "convert_lunar_to_solar",
            json!({ "day": 1, "month": 1, "year": 2024, "is_leap_month": "yes" }),
        );
        assert_eq!(
            err2,
            Some(-32602),
            "is_leap_month string must return -32602"
        );

        let (_, err3) = call_tool(
            reg.as_ref(),
            "convert_lunar_to_solar",
            json!({ "day": 1, "month": 1, "year": 2024, "timezone": "bad" }),
        );
        assert_eq!(
            err3,
            Some(-32602),
            "convert_lunar_to_solar timezone string must return -32602"
        );

        let (_, err4) = call_tool(
            reg.as_ref(),
            "get_year_holidays",
            json!({ "year": 2026, "timezone": "bad" }),
        );
        assert_eq!(
            err4,
            Some(-32602),
            "get_year_holidays timezone string must return -32602"
        );

        let (_, err5) = call_tool(
            reg.as_ref(),
            "check_vietnam_holiday",
            json!({ "date": "2026-04-27", "timezone": "bad" }),
        );
        assert_eq!(
            err5,
            Some(-32602),
            "check_vietnam_holiday timezone string must return -32602"
        );

        kernel.shutdown().await;
    }

    #[tokio::test]
    async fn test_mcp_tools_handlers_and_resource_uris() {
        let (kernel, reg) = mount_mcp_kernel().await;

        let (today_res, today_err) = call_tool(reg.as_ref(), "get_today_lunar", json!({}));
        assert!(today_err.is_none());
        let today_txt = today_res.unwrap()["content"][0]["text"]
            .as_str()
            .unwrap()
            .to_string();
        let today_val: Value = serde_json::from_str(&today_txt).unwrap();
        assert_eq!(today_val["timezone"], "UTC+7 (Indochina Time)");

        let (dmy_res, dmy_err) = call_tool(
            reg.as_ref(),
            "convert_solar_to_lunar",
            json!({ "day": 10, "month": 2, "year": 2024 }),
        );
        assert!(dmy_err.is_none());
        let dmy_val: Value =
            serde_json::from_str(dmy_res.unwrap()["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(dmy_val["lunar_date"]["day"], 1);
        assert_eq!(dmy_val["lunar_date"]["month"], 1);
        assert_eq!(dmy_val["lunar_date"]["year"], 2024);

        let (hol_res, hol_err) = call_tool(
            reg.as_ref(),
            "check_vietnam_holiday",
            json!({ "date": "2026-08-22" }),
        );
        assert!(hol_err.is_none());
        let hol_val: Value =
            serde_json::from_str(hol_res.unwrap()["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(hol_val["is_holiday"], false);

        let ok_read = reg
            .dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(2)),
                method: "resources/read".into(),
                params: Some(json!({ "uri": "calendar://holidays/2026" })),
            })
            .unwrap();
        assert!(ok_read.error.is_none());
        let ok_read_txt = ok_read.result.unwrap()["contents"][0]["text"]
            .as_str()
            .unwrap()
            .to_string();
        let ok_read_json: Value = serde_json::from_str(&ok_read_txt).unwrap();
        assert_eq!(ok_read_json["configured"], true);

        let bad_read = reg
            .dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(3)),
                method: "resources/read".into(),
                params: Some(json!({ "uri": "calendar://holidays/not-a-year" })),
            })
            .unwrap();
        assert_eq!(bad_read.error.unwrap().code, -32602);

        kernel.shutdown().await;
    }
}
