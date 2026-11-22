//! Thin MCP Cordis plugins exposing all Vietnamese Lunar niche tools and resource templates.

use std::sync::Arc;

use chrono::{Duration, NaiveDate, Utc};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};

use crate::contracts::astrology::{AstrologyQuery, AstrologyService, ChartSystem, Gender};
use crate::contracts::canchi::CanChiSolarService;
use crate::contracts::mcp::{McpError, McpRegistryService, McpResourceTemplateSpec, McpToolSpec};
use crate::contracts::recurrence::{LunarRecurrenceService, RecurrenceRequest};
use crate::contracts::retail::{RetailCalendarService, RetailCategory};
use crate::contracts::trach_nhat::{AuspiciousSearchQuery, Occasion, TrachNhatService};
use crate::kernel::context::Context;
use crate::kernel::effect::BoxFuture;
use crate::kernel::error::KernelError;
use crate::kernel::fiber::{Plugin, PluginMeta};
use crate::plugins::http_calendar::parse_ymd_ascii;

fn decode_args<T: DeserializeOwned>(args: Value) -> Result<T, McpError> {
    serde_json::from_value(args).map_err(|e| McpError::InvalidParams(e.to_string()))
}

fn to_mcp_value<T: serde::Serialize>(val: &T) -> Result<Value, McpError> {
    serde_json::to_value(val).map_err(|e| McpError::Internal(e.to_string()))
}

fn parse_req_str<'a>(args: &'a Value, key: &str) -> Result<&'a str, McpError> {
    args.get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| McpError::InvalidParams(format!("Missing required string '{key}'")))
}

fn parse_opt_u8(args: &Value, key: &str, default: u8) -> Result<u8, McpError> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(default),
        Some(v) => {
            let n = v
                .as_i64()
                .ok_or_else(|| McpError::InvalidParams(format!("'{key}' must be an integer")))?;
            u8::try_from(n)
                .map_err(|_| McpError::InvalidParams(format!("'{key}' out of 0..=255 bounds: {n}")))
        }
    }
}

fn parse_req_i32(args: &Value, key: &str) -> Result<i32, McpError> {
    let raw = args
        .get(key)
        .and_then(|v| v.as_i64())
        .ok_or_else(|| McpError::InvalidParams(format!("Missing required integer '{key}'")))?;
    i32::try_from(raw)
        .map_err(|_| McpError::InvalidParams(format!("'{key}' out of i32 bounds: {raw}")))
}

fn parse_opt_i32(args: &Value, key: &str) -> Result<Option<i32>, McpError> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => {
            let raw = v
                .as_i64()
                .ok_or_else(|| McpError::InvalidParams(format!("'{key}' must be an integer")))?;
            i32::try_from(raw)
                .map(Some)
                .map_err(|_| McpError::InvalidParams(format!("'{key}' out of i32 bounds: {raw}")))
        }
    }
}

fn parse_opt_f64(args: &Value, key: &str, default: f64) -> Result<f64, McpError> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(default),
        Some(v) => {
            let num = v
                .as_f64()
                .ok_or_else(|| McpError::InvalidParams(format!("'{key}' must be a number")))?;
            if !num.is_finite() || !(-12.0..=14.0).contains(&num) {
                return Err(McpError::InvalidParams(format!("Invalid timezone: {num}")));
            }
            Ok(num)
        }
    }
}

fn parse_date_arg(raw: &str) -> Result<NaiveDate, McpError> {
    let (y, m, d) = parse_ymd_ascii(raw).ok_or_else(|| {
        McpError::InvalidParams(format!("Invalid date '{raw}'. Expected YYYY-MM-DD"))
    })?;
    let mu = u32::try_from(m).map_err(|_| McpError::InvalidParams(raw.to_string()))?;
    let du = u32::try_from(d).map_err(|_| McpError::InvalidParams(raw.to_string()))?;
    NaiveDate::from_ymd_opt(y, mu, du)
        .ok_or_else(|| McpError::InvalidParams(format!("Invalid calendar date '{raw}'")))
}

// ---------- 1. McpCanChiPlugin ----------

/// Cordis MCP plugin exposing Can Chi & 24 Solar Terms tools and resource templates.
#[derive(Debug, Default)]
pub struct McpCanChiPlugin;

impl Plugin for McpCanChiPlugin {
    fn meta(&self) -> PluginMeta {
        PluginMeta::new("mcp_canchi")
            .requires::<dyn CanChiSolarService>()
            .requires::<dyn McpRegistryService>()
    }

    fn mount<'a>(&'a self, ctx: Context, _config: Value) -> BoxFuture<'a, Result<(), KernelError>> {
        Box::pin(async move {
            let svc = ctx.inject::<dyn CanChiSolarService>()?;
            let reg = ctx.inject::<dyn McpRegistryService>()?;

            // Tool: get_can_chi
            {
                let s = Arc::clone(&svc);
                ctx.on_dispose_sync(reg.register_tool(McpToolSpec {
                    name: "get_can_chi".into(),
                    description: "Compute bilingual Year, Month (Lunar & BaZi Tiết Khí), Day, Hour (Ngũ Thử Độn) Can Chi pillars and Nạp Âm.".into(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "date": { "type": "string", "description": "Gregorian date YYYY-MM-DD" },
                            "hour": { "type": "integer", "description": "Local hour (0-23, default 12)" },
                            "minute": { "type": "integer", "description": "Local minute (0-59, default 0)" },
                            "timezone": { "type": "number", "description": "UTC offset hours (default 7.0)" }
                        },
                        "required": ["date"]
                    }),
                    handler: Arc::new(move |args| {
                        let date = parse_date_arg(parse_req_str(&args, "date")?)?;
                        let hour = parse_opt_u8(&args, "hour", 12)?;
                        let minute = parse_opt_u8(&args, "minute", 0)?;
                        let tz = parse_opt_f64(&args, "timezone", 7.0)?;
                        let res = s
                            .full_can_chi(date, hour, minute, tz)
                            .map_err(|e| McpError::InvalidParams(e.to_string()))?;
                        to_mcp_value(&res)
                    }),
                }));
            }

            // Tool: get_solar_terms
            {
                let s = Arc::clone(&svc);
                ctx.on_dispose_sync(reg.register_tool(McpToolSpec {
                    name: "get_solar_terms".into(),
                    description: "List all 24 astronomical solar terms (24 Tiết Khí) with exact local timestamps for a Gregorian year.".into(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "year": { "type": "integer", "description": "Gregorian year (e.g. 2024)" },
                            "timezone": { "type": "number", "description": "UTC offset hours (default 7.0)" }
                        },
                        "required": ["year"]
                    }),
                    handler: Arc::new(move |args| {
                        let year = parse_req_i32(&args, "year")?;
                        let tz = parse_opt_f64(&args, "timezone", 7.0)?;
                        let terms = s
                            .solar_terms_for_year(year, tz)
                            .map_err(|e| McpError::InvalidParams(e.to_string()))?;
                        Ok(json!({ "year": year, "timezone": tz, "solar_terms": terms }))
                    }),
                }));
            }

            // Tool: get_active_solar_term
            {
                let s = Arc::clone(&svc);
                ctx.on_dispose_sync(reg.register_tool(McpToolSpec {
                    name: "get_active_solar_term".into(),
                    description: "Find instantaneous ecliptic solar longitude plus current and next Tiết Khí at a timestamp.".into(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "date": { "type": "string", "description": "Gregorian date YYYY-MM-DD" },
                            "hour": { "type": "integer", "description": "Local hour (0-23, default 12)" },
                            "minute": { "type": "integer", "description": "Local minute (0-59, default 0)" },
                            "timezone": { "type": "number", "description": "UTC offset hours (default 7.0)" }
                        },
                        "required": ["date"]
                    }),
                    handler: Arc::new(move |args| {
                        let date = parse_date_arg(parse_req_str(&args, "date")?)?;
                        let hour = parse_opt_u8(&args, "hour", 12)?;
                        let minute = parse_opt_u8(&args, "minute", 0)?;
                        let tz = parse_opt_f64(&args, "timezone", 7.0)?;
                        let res = s
                            .active_solar_term(date, hour, minute, tz)
                            .map_err(|e| McpError::InvalidParams(e.to_string()))?;
                        to_mcp_value(&res)
                    }),
                }));
            }

            // Resource Template: canchi://solar-terms/{year}
            {
                let s = Arc::clone(&svc);
                ctx.on_dispose_sync(reg.register_resource_template(McpResourceTemplateSpec {
                    uri_template: "canchi://solar-terms/{year}".into(),
                    name: "Solar Terms by Year".into(),
                    description: "All 24 astronomical solar terms (24 Tiết Khí) in UTC+7 for a Gregorian year.".into(),
                    mime_type: "application/json".into(),
                    prefix: "canchi://solar-terms/".into(),
                    handler: Arc::new(move |suffix| {
                        let year: i32 = suffix.trim().parse().map_err(|_| {
                            McpError::InvalidParams(format!("Invalid solar-terms year '{suffix}'"))
                        })?;
                        let terms = s
                            .solar_terms_for_year(year, 7.0)
                            .map_err(|e| McpError::InvalidParams(e.to_string()))?;
                        Ok(json!({ "year": year, "solar_terms": terms }))
                    }),
                }));
            }

            Ok(())
        })
    }
}

// ---------- 2. McpAstrologyPlugin ----------

/// Cordis MCP plugin exposing `generate_bazi_chart` and `generate_tuvi_chart`.
#[derive(Debug, Default)]
pub struct McpAstrologyPlugin;

impl Plugin for McpAstrologyPlugin {
    fn meta(&self) -> PluginMeta {
        PluginMeta::new("mcp_astrology")
            .requires::<dyn AstrologyService>()
            .requires::<dyn McpRegistryService>()
    }

    fn mount<'a>(&'a self, ctx: Context, _config: Value) -> BoxFuture<'a, Result<(), KernelError>> {
        Box::pin(async move {
            let svc = ctx.inject::<dyn AstrologyService>()?;
            let reg = ctx.inject::<dyn McpRegistryService>()?;

            // Tool: generate_bazi_chart
            {
                let s = Arc::clone(&svc);
                ctx.on_dispose_sync(reg.register_tool(McpToolSpec {
                    name: "generate_bazi_chart".into(),
                    description: "Generate deterministic BaZi (Four Pillars / Tứ Trụ) birth chart with Tiết Khí boundaries, Hidden Stems, Ten Gods, Five Elements, and Đại Vận.".into(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "date": { "type": "string", "description": "Birth date YYYY-MM-DD" },
                            "hour": { "type": "integer", "description": "Birth hour (0-23)" },
                            "minute": { "type": "integer", "description": "Birth minute (0-59, default 0)" },
                            "gender": { "type": "string", "enum": ["male", "female"] },
                            "is_lunar": { "type": "boolean", "default": false },
                            "is_leap_month": { "type": "boolean", "default": false },
                            "time_zone": { "type": "number", "default": 7.0 }
                        },
                        "required": ["date", "hour", "gender"]
                    }),
                    handler: Arc::new(move |args| {
                        let mut q: AstrologyQuery = decode_args(args)?;
                        q.system = ChartSystem::Bazi;
                        let res = s
                            .generate_chart(&q)
                            .map_err(|e| McpError::InvalidParams(e.to_string()))?;
                        to_mcp_value(&res)
                    }),
                }));
            }

            // Tool: generate_tuvi_chart
            {
                let s = Arc::clone(&svc);
                ctx.on_dispose_sync(reg.register_tool(McpToolSpec {
                    name: "generate_tuvi_chart".into(),
                    description: "Generate deterministic Vietnamese Tử Vi (Nam Phái Core) 12-Palace star chart (~50 major stars, Cục, Tứ Hóa, Thái Tuế, Trường Sinh, Tuần/Triệt, Đại Hạn).".into(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "date": { "type": "string", "description": "Birth date YYYY-MM-DD" },
                            "hour": { "type": "integer", "description": "Birth hour (0-23)" },
                            "minute": { "type": "integer", "description": "Birth minute (0-59, default 0)" },
                            "gender": { "type": "string", "enum": ["male", "female"] },
                            "system": { "type": "string", "enum": ["tuvi", "both"], "default": "tuvi" },
                            "is_lunar": { "type": "boolean", "default": false },
                            "is_leap_month": { "type": "boolean", "default": false },
                            "time_zone": { "type": "number", "default": 7.0 }
                        },
                        "required": ["date", "hour", "gender"]
                    }),
                    handler: Arc::new(move |mut args| {
                        if let Some(obj) = args.as_object_mut() {
                            if !obj.contains_key("system") {
                                obj.insert("system".to_string(), json!("tuvi"));
                            }
                        }
                        let mut q: AstrologyQuery = decode_args(args)?;
                        if q.system == ChartSystem::Bazi {
                            let _ = Gender::Male;
                            q.system = ChartSystem::Tuvi;
                        }
                        let res = s
                            .generate_chart(&q)
                            .map_err(|e| McpError::InvalidParams(e.to_string()))?;
                        to_mcp_value(&res)
                    }),
                }));
            }

            Ok(())
        })
    }
}

// ---------- 3. McpTrachNhatPlugin ----------

/// Cordis MCP plugin exposing `analyze_auspicious_date` and `find_auspicious_dates`.
#[derive(Debug, Default)]
pub struct McpTrachNhatPlugin;

impl Plugin for McpTrachNhatPlugin {
    fn meta(&self) -> PluginMeta {
        PluginMeta::new("mcp_trach_nhat")
            .requires::<dyn TrachNhatService>()
            .requires::<dyn McpRegistryService>()
    }

    fn mount<'a>(&'a self, ctx: Context, _config: Value) -> BoxFuture<'a, Result<(), KernelError>> {
        Box::pin(async move {
            let svc = ctx.inject::<dyn TrachNhatService>()?;
            let reg = ctx.inject::<dyn McpRegistryService>()?;

            // Tool: analyze_auspicious_date
            {
                let s = Arc::clone(&svc);
                ctx.on_dispose_sync(reg.register_tool(McpToolSpec {
                    name: "analyze_auspicious_date".into(),
                    description: "Analyze a date using Vietnamese Trạch Nhật (Hoàng Đạo hours/day, 12 Trực, 28 Tú, major taboos, Tam Tai/Kim Lâu/Hoang Ốc/Xung Tuổi).".into(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "date": { "type": "string", "description": "Gregorian date YYYY-MM-DD" },
                            "occasion": { "type": "string", "enum": ["wedding", "groundbreaking", "contract_signing", "moving_in", "grand_opening", "general"] },
                            "owner_birth_year": { "type": "integer" },
                            "partner_birth_year": { "type": "integer" },
                            "timezone": { "type": "number", "default": 7.0 }
                        },
                        "required": ["date"]
                    }),
                    handler: Arc::new(move |args| {
                        let date = parse_date_arg(parse_req_str(&args, "date")?)?;
                        let occasion: Occasion = match args.get("occasion") {
                            None | Some(Value::Null) => Occasion::General,
                            Some(v) => serde_json::from_value(v.clone())
                                .map_err(|e| McpError::InvalidParams(e.to_string()))?,
                        };
                        let owner = parse_opt_i32(&args, "owner_birth_year")?;
                        let partner = parse_opt_i32(&args, "partner_birth_year")?;
                        let tz = parse_opt_f64(&args, "timezone", 7.0)?;
                        let res = s
                            .analyze_date(date, occasion, owner, partner, tz)
                            .map_err(|e| McpError::InvalidParams(e.to_string()))?;
                        to_mcp_value(&res)
                    }),
                }));
            }

            // Tool: find_auspicious_dates
            {
                let s = Arc::clone(&svc);
                ctx.on_dispose_sync(reg.register_tool(McpToolSpec {
                    name: "find_auspicious_dates".into(),
                    description: "Search and rank top auspicious dates across a bounded range (<= 366 days) by occasion and personal birth years.".into(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "start_date": { "type": "string" },
                            "end_date": { "type": "string" },
                            "occasion": { "type": "string", "enum": ["wedding", "groundbreaking", "contract_signing", "moving_in", "grand_opening", "general"] },
                            "owner_birth_year": { "type": "integer" },
                            "partner_birth_year": { "type": "integer" },
                            "min_score": { "type": "integer" },
                            "limit": { "type": "integer" },
                            "time_zone": { "type": "number", "default": 7.0 }
                        },
                        "required": ["start_date", "end_date", "occasion"]
                    }),
                    handler: Arc::new(move |args| {
                        let q: AuspiciousSearchQuery = decode_args(args)?;
                        let res = s
                            .find_auspicious_dates(&q)
                            .map_err(|e| McpError::InvalidParams(e.to_string()))?;
                        to_mcp_value(&res)
                    }),
                }));
            }

            Ok(())
        })
    }
}

// ---------- 4. McpRecurrencePlugin ----------

/// Cordis MCP plugin exposing stateless lunar recurrence expansion and RFC 5545 `.ics` export.
#[derive(Debug, Default)]
pub struct McpRecurrencePlugin;

impl Plugin for McpRecurrencePlugin {
    fn meta(&self) -> PluginMeta {
        PluginMeta::new("mcp_recurrence")
            .requires::<dyn LunarRecurrenceService>()
            .requires::<dyn McpRegistryService>()
    }

    fn mount<'a>(&'a self, ctx: Context, _config: Value) -> BoxFuture<'a, Result<(), KernelError>> {
        Box::pin(async move {
            let svc = ctx.inject::<dyn LunarRecurrenceService>()?;
            let reg = ctx.inject::<dyn McpRegistryService>()?;

            // Tool: expand_lunar_recurrence
            {
                let s = Arc::clone(&svc);
                ctx.on_dispose_sync(reg.register_tool(McpToolSpec {
                    name: "expand_lunar_recurrence".into(),
                    description: "Expand stateless lunar recurring dates (anniversaries, Rằm/Mùng 1, vegetarian presets) over <= 50 years with 29-day & leap-month policies.".into(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "title": { "type": "string" },
                            "frequency": { "type": "string", "enum": ["monthly", "yearly"] },
                            "preset": { "type": "string", "enum": ["new_and_full_moon", "vegetarian_four_days", "vegetarian_six_days", "vegetarian_ten_days"] },
                            "lunar_day": { "type": "integer" },
                            "lunar_month": { "type": "integer" },
                            "start_date": { "type": "string" },
                            "end_date": { "type": "string" },
                            "day30_policy": { "type": "string", "enum": ["skip", "last_day_of_month"] },
                            "leap_month_policy": { "type": "string", "enum": ["primary_only", "include_leap", "leap_only"] },
                            "time_zone": { "type": "number", "default": 7.0 }
                        },
                        "required": ["start_date", "end_date"]
                    }),
                    handler: Arc::new(move |args| {
                        let req: RecurrenceRequest = decode_args(args)?;
                        let res = s
                            .expand_occurrences(&req)
                            .map_err(|e| McpError::InvalidParams(e.to_string()))?;
                        to_mcp_value(&res)
                    }),
                }));
            }

            // Tool: export_lunar_ics
            {
                let s = Arc::clone(&svc);
                ctx.on_dispose_sync(reg.register_tool(McpToolSpec {
                    name: "export_lunar_ics".into(),
                    description: "Generate RFC 5545 compliant .ics iCalendar feed (CRLF + <=75-octet UTF-8 folding + optional VALARM) for lunar recurrences.".into(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "title": { "type": "string" },
                            "description": { "type": "string" },
                            "frequency": { "type": "string", "enum": ["monthly", "yearly"] },
                            "preset": { "type": "string", "enum": ["new_and_full_moon", "vegetarian_four_days", "vegetarian_six_days", "vegetarian_ten_days"] },
                            "lunar_day": { "type": "integer" },
                            "lunar_month": { "type": "integer" },
                            "start_date": { "type": "string" },
                            "end_date": { "type": "string" },
                            "day30_policy": { "type": "string", "enum": ["skip", "last_day_of_month"] },
                            "leap_month_policy": { "type": "string", "enum": ["primary_only", "include_leap", "leap_only"] },
                            "reminder_minutes": { "type": "integer" },
                            "time_zone": { "type": "number", "default": 7.0 }
                        },
                        "required": ["start_date", "end_date"]
                    }),
                    handler: Arc::new(move |args| {
                        let req: RecurrenceRequest = decode_args(args)?;
                        let ics = s
                            .export_ics(&req)
                            .map_err(|e| McpError::InvalidParams(e.to_string()))?;
                        Ok(json!({ "content_type": "text/calendar", "ics": ics }))
                    }),
                }));
            }

            Ok(())
        })
    }
}

// ---------- 5. McpRetailPlugin ----------

/// Cordis MCP plugin exposing Vietnam retail peak tools and resource template `retail://calendar/{year}`.
#[derive(Debug, Default)]
pub struct McpRetailPlugin;

impl Plugin for McpRetailPlugin {
    fn meta(&self) -> PluginMeta {
        PluginMeta::new("mcp_retail")
            .requires::<dyn RetailCalendarService>()
            .requires::<dyn McpRegistryService>()
    }

    fn mount<'a>(&'a self, ctx: Context, _config: Value) -> BoxFuture<'a, Result<(), KernelError>> {
        Box::pin(async move {
            let svc = ctx.inject::<dyn RetailCalendarService>()?;
            let reg = ctx.inject::<dyn McpRegistryService>()?;

            // Tool: get_retail_calendar
            {
                let s = Arc::clone(&svc);
                ctx.on_dispose_sync(reg.register_tool(McpToolSpec {
                    name: "get_retail_calendar".into(),
                    description: "Resolve Vietnam lunar retail peaks, monthly Mùng 1/Rằm cycles, and campaign lead-time milestones for a year.".into(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "year": { "type": "integer" },
                            "category": { "type": "string", "enum": ["festival", "religious", "shopping", "food_beverage", "family_gift"] },
                            "timezone": { "type": "number", "default": 7.0 }
                        },
                        "required": ["year"]
                    }),
                    handler: Arc::new(move |args| {
                        let year = parse_req_i32(&args, "year")?;
                        let category: Option<RetailCategory> = match args.get("category") {
                            None | Some(Value::Null) => None,
                            Some(v) => Some(
                                serde_json::from_value(v.clone())
                                    .map_err(|e| McpError::InvalidParams(e.to_string()))?,
                            ),
                        };
                        let tz = parse_opt_f64(&args, "timezone", 7.0)?;
                        let res = s
                            .retail_events_for_year(year, category, tz)
                            .map_err(|e| McpError::InvalidParams(e.to_string()))?;
                        to_mcp_value(&res)
                    }),
                }));
            }

            // Tool: get_upcoming_retail_peaks
            {
                let s = Arc::clone(&svc);
                ctx.on_dispose_sync(reg.register_tool(McpToolSpec {
                    name: "get_upcoming_retail_peaks".into(),
                    description: "List upcoming Vietnam retail peaks within 1..=366 days from a start date with prep/early-bird/peak dates.".into(),
                    input_schema: json!({
                        "type": "object",
                        "properties": {
                            "from_date": { "type": "string", "description": "Optional YYYY-MM-DD (defaults to today UTC+7)" },
                            "days": { "type": "integer", "description": "Lookahead days (1..=366, default 60)" },
                            "category": { "type": "string", "enum": ["festival", "religious", "shopping", "food_beverage", "family_gift"] },
                            "timezone": { "type": "number", "default": 7.0 }
                        }
                    }),
                    handler: Arc::new(move |args| {
                        let from = match args.get("from_date").and_then(|v| v.as_str()) {
                            Some(raw) => parse_date_arg(raw)?,
                            None => (Utc::now() + Duration::hours(7)).date_naive(),
                        };
                        let days = parse_opt_i32(&args, "days")?.unwrap_or(60);
                        let category: Option<RetailCategory> = match args.get("category") {
                            None | Some(Value::Null) => None,
                            Some(v) => Some(
                                serde_json::from_value(v.clone())
                                    .map_err(|e| McpError::InvalidParams(e.to_string()))?,
                            ),
                        };
                        let tz = parse_opt_f64(&args, "timezone", 7.0)?;
                        let res = s
                            .upcoming_retail_peaks(from, days, category, tz)
                            .map_err(|e| McpError::InvalidParams(e.to_string()))?;
                        to_mcp_value(&res)
                    }),
                }));
            }

            // Resource Template: retail://calendar/{year}
            {
                let s = Arc::clone(&svc);
                ctx.on_dispose_sync(reg.register_resource_template(McpResourceTemplateSpec {
                    uri_template: "retail://calendar/{year}".into(),
                    name: "Vietnam Retail Calendar by Year".into(),
                    description: "All configured Vietnam retail peaks & monthly Mùng 1/Rằm cycles in UTC+7 for a Gregorian year.".into(),
                    mime_type: "application/json".into(),
                    prefix: "retail://calendar/".into(),
                    handler: Arc::new(move |suffix| {
                        let year: i32 = suffix.trim().parse().map_err(|_| {
                            McpError::InvalidParams(format!("Invalid retail year '{suffix}'"))
                        })?;
                        let res = s
                            .retail_events_for_year(year, None, 7.0)
                            .map_err(|e| McpError::InvalidParams(e.to_string()))?;
                        to_mcp_value(&res)
                    }),
                }));
            }

            Ok(())
        })
    }
}
