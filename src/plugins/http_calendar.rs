//! `HttpCalendarPlugin`: REST handlers for Vietnamese solar/lunar date conversion.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use chrono::{Datelike, Duration, NaiveDate, Utc};
use serde::Deserialize;
use serde_json::Value;
use utoipa::{IntoParams, OpenApi};

use crate::contracts::calendar::CalendarService;
use crate::contracts::http::HttpRouterService;
use crate::contracts::models::{DateMonthYear, ErrorResponse};
use crate::kernel::context::Context;
use crate::kernel::effect::BoxFuture;
use crate::kernel::error::KernelError;
use crate::kernel::fiber::{Plugin, PluginMeta};
use crate::plugins::http_server::{AppError, JsonQuery};

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct LunarQuery {
    /// Solar day (`1..=31`).
    pub dd: u32,
    /// Solar month (`1..=12`, accepts `mm` or `MM`).
    #[serde(alias = "mm", alias = "MM")]
    pub mm: u32,
    /// Solar year.
    pub yyyy: i32,
    /// Timezone offset in hours (accepts `timezone` or `timeZone`, default `7.0`).
    #[serde(alias = "timeZone", alias = "timezone")]
    #[param(rename = "timezone")]
    pub time_zone: Option<f64>,
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ConvertSolarToLunarQuery {
    /// Timezone offset in hours (accepts `timezone` or `timeZone`, default `7.0`).
    #[serde(alias = "timeZone", alias = "timezone")]
    #[param(rename = "timezone")]
    pub time_zone: Option<f64>,
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ConvertLunarToSolarQuery {
    /// Set `true` when converting a date inside a leap month (*tháng nhuận*).
    pub leap: Option<bool>,
    /// Timezone offset in hours (accepts `timezone` or `timeZone`, default `7.0`).
    #[serde(alias = "timeZone", alias = "timezone")]
    #[param(rename = "timezone")]
    pub time_zone: Option<f64>,
}

/// Zero-allocation ASCII `YYYY-MM-DD` parser supporting optional leading `-` years.
#[inline]
pub fn parse_ymd_ascii(input: &str) -> Option<(i32, i32, i32)> {
    let bytes = input.as_bytes();
    if bytes.is_empty() {
        return None;
    }

    let (neg, rest) = if bytes[0] == b'-' {
        (true, &bytes[1..])
    } else {
        (false, bytes)
    };

    let d1 = rest.iter().position(|&b| b == b'-')?;
    if d1 == 0 {
        return None;
    }
    let year_abs = parse_uint_slice(&rest[..d1])?;
    let year = if neg { -year_abs } else { year_abs };

    let after_y = &rest[d1 + 1..];
    let d2 = after_y.iter().position(|&b| b == b'-')?;
    if d2 == 0 || d2 > 2 {
        return None;
    }
    let month = parse_uint_slice(&after_y[..d2])?;

    let day_slice = &after_y[d2 + 1..];
    if day_slice.is_empty() || day_slice.len() > 2 {
        return None;
    }
    let day = parse_uint_slice(day_slice)?;

    Some((year, month, day))
}

#[inline]
fn parse_uint_slice(bytes: &[u8]) -> Option<i32> {
    if bytes.is_empty() {
        return None;
    }
    let mut acc: i32 = 0;
    for &b in bytes {
        if !b.is_ascii_digit() {
            return None;
        }
        acc = acc.checked_mul(10)?.checked_add(i32::from(b - b'0'))?;
    }
    Some(acc)
}

#[derive(OpenApi)]
#[openapi(
    paths(home, get_lunar, get_solar_to_lunar, get_lunar_to_solar),
    components(schemas(DateMonthYear, ErrorResponse))
)]
struct CalendarApiDoc;

/// Get today's lunar date anchored to Vietnam Indochina Time (`UTC+7`).
#[utoipa::path(
    get,
    path = "/",
    responses(
        (status = 200, description = "Get today's lunar date in Vietnam (UTC+7)", body = DateMonthYear),
        (status = 400, description = "Calendar conversion error", body = ErrorResponse)
    )
)]
pub async fn home(
    State(cal): State<Arc<dyn CalendarService>>,
) -> Result<Json<DateMonthYear>, AppError> {
    let now_vietnam = (Utc::now() + Duration::hours(7)).date_naive();
    let lunar = cal.to_lunar(now_vietnam, 7.0)?;
    Ok(Json(DateMonthYear {
        dd: lunar.day,
        mm: lunar.month,
        yyyy: lunar.year,
        leap: if lunar.is_leap { Some(true) } else { None },
    }))
}

/// Convert a Gregorian solar date to Vietnamese lunar format using query parameters.
#[utoipa::path(
    get,
    path = "/lunar",
    params(LunarQuery),
    responses(
        (status = 200, description = "Convert a solar date to a lunar date via query params", body = DateMonthYear),
        (status = 400, description = "Invalid date format", body = ErrorResponse)
    )
)]
pub async fn get_lunar(
    State(cal): State<Arc<dyn CalendarService>>,
    JsonQuery(q): JsonQuery<LunarQuery>,
) -> Result<Json<DateMonthYear>, AppError> {
    let date = NaiveDate::from_ymd_opt(q.yyyy, q.mm, q.dd).ok_or_else(|| {
        AppError::BadRequest(format!(
            "Invalid solar date: {:02}/{:02}/{}",
            q.dd, q.mm, q.yyyy
        ))
    })?;

    let tz = q.time_zone.unwrap_or(7.0);
    let lunar = cal.to_lunar(date, tz)?;
    Ok(Json(DateMonthYear {
        dd: lunar.day,
        mm: lunar.month,
        yyyy: lunar.year,
        leap: if lunar.is_leap { Some(true) } else { None },
    }))
}

/// Convert a Gregorian solar date (`YYYY-MM-DD`) to Vietnamese lunar format.
#[utoipa::path(
    get,
    path = "/convert/solar-to-lunar/{date}",
    params(
        ("date" = String, Path, description = "Solar date in ISO format YYYY-MM-DD (e.g. 2015-09-12)"),
        ConvertSolarToLunarQuery
    ),
    responses(
        (status = 200, description = "Convert solar date to lunar date", body = DateMonthYear),
        (status = 400, description = "Invalid date format", body = ErrorResponse)
    )
)]
pub async fn get_solar_to_lunar(
    State(cal): State<Arc<dyn CalendarService>>,
    Path(date_str): Path<String>,
    JsonQuery(q): JsonQuery<ConvertSolarToLunarQuery>,
) -> Result<Json<DateMonthYear>, AppError> {
    let date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").map_err(|_| {
        AppError::BadRequest("Expected ISO date format: YYYY-MM-DD (e.g. 2015-09-12)".to_string())
    })?;

    let tz = q.time_zone.unwrap_or(7.0);
    let lunar = cal.to_lunar(date, tz)?;
    Ok(Json(DateMonthYear {
        dd: lunar.day,
        mm: lunar.month,
        yyyy: lunar.year,
        leap: if lunar.is_leap { Some(true) } else { None },
    }))
}

/// Convert a Vietnamese lunar date (`YYYY-MM-DD`) to Gregorian solar format.
#[utoipa::path(
    get,
    path = "/convert/lunar-to-solar/{date}",
    params(
        ("date" = String, Path, description = "Lunar date in format YYYY-MM-DD (e.g. 2015-07-30)"),
        ConvertLunarToSolarQuery
    ),
    responses(
        (status = 200, description = "Convert lunar date to solar date", body = DateMonthYear),
        (status = 400, description = "Invalid lunar date or leap month", body = ErrorResponse)
    )
)]
pub async fn get_lunar_to_solar(
    State(cal): State<Arc<dyn CalendarService>>,
    Path(date_str): Path<String>,
    JsonQuery(q): JsonQuery<ConvertLunarToSolarQuery>,
) -> Result<Json<DateMonthYear>, AppError> {
    let (yyyy, mm, dd) = parse_ymd_ascii(&date_str).ok_or_else(|| {
        AppError::BadRequest("Expected format: YYYY-MM-DD (e.g. 2015-07-30)".to_string())
    })?;

    let tz = q.time_zone.unwrap_or(7.0);
    let is_leap = q.leap.unwrap_or(false);
    let solar_date = cal.to_solar(dd, mm, yyyy, is_leap, tz)?;

    Ok(Json(DateMonthYear {
        dd: solar_date.day() as i32,
        mm: solar_date.month() as i32,
        yyyy: solar_date.year(),
        leap: None,
    }))
}

/// Cordis plugin mounting calendar conversion REST endpoints onto `HttpRouterService`.
#[derive(Debug, Default)]
pub struct HttpCalendarPlugin;

impl Plugin for HttpCalendarPlugin {
    fn meta(&self) -> PluginMeta {
        PluginMeta::new("http_calendar")
            .requires::<dyn CalendarService>()
            .requires::<dyn HttpRouterService>()
    }

    fn mount<'a>(&'a self, ctx: Context, _config: Value) -> BoxFuture<'a, Result<(), KernelError>> {
        Box::pin(async move {
            let cal = ctx.inject::<dyn CalendarService>()?;
            let router_svc = ctx.inject::<dyn HttpRouterService>()?;

            let router = Router::new()
                .route("/", get(home))
                .route("/lunar", get(get_lunar))
                .route("/convert/solar-to-lunar/{date}", get(get_solar_to_lunar))
                .route("/convert/lunar-to-solar/{date}", get(get_lunar_to_solar))
                .with_state(cal);

            let disposer = router_svc.mount_routes(router, Some(CalendarApiDoc::openapi()));
            ctx.on_dispose_sync(disposer);
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ymd_ascii() {
        assert_eq!(parse_ymd_ascii("2015-07-30"), Some((2015, 7, 30)));
        assert_eq!(parse_ymd_ascii("2024-2-9"), Some((2024, 2, 9)));
        assert_eq!(parse_ymd_ascii("-100-01-05"), Some((-100, 1, 5)));
        assert_eq!(parse_ymd_ascii("20150730"), None);
        assert_eq!(parse_ymd_ascii("2015-aa-30"), None);
        assert_eq!(parse_ymd_ascii("2015-07-30-01"), None);
        assert_eq!(parse_ymd_ascii(""), None);
    }
}
