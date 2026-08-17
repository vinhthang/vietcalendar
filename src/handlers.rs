use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::models::DateMonthYear;
use crate::services::{is_vietnam_holiday, to_lunar};

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Debug)]
pub enum AppError {
    BadRequest(String),
    NotFound(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
        };
        (status, Json(ErrorResponse { error: message })).into_response()
    }
}

#[derive(Deserialize, IntoParams)]
#[allow(non_snake_case)]
pub struct LunarQuery {
    pub dd: u32,
    #[serde(alias = "mm", alias = "MM")]
    pub mm: u32,
    pub yyyy: i32,
    #[serde(alias = "timeZone", alias = "timezone")]
    pub time_zone: Option<f64>,
}

#[derive(Deserialize, IntoParams)]
#[allow(non_snake_case)]
pub struct HolidayQuery {
    pub dd: u32,
    #[serde(alias = "mm", alias = "MM")]
    pub mm: u32,
    pub yyyy: i32,
}

#[utoipa::path(
    get,
    path = "/",
    responses(
        (status = 200, description = "Get today's lunar date in Vietnam (UTC+7)", body = DateMonthYear)
    )
)]
pub async fn home() -> Json<DateMonthYear> {
    let now_vietnam = (Utc::now() + chrono::Duration::hours(7)).date_naive();
    let lunar_date = to_lunar(now_vietnam, 7.0);
    Json(DateMonthYear {
        dd: lunar_date.day,
        mm: lunar_date.month,
        yyyy: lunar_date.year,
        leap: if lunar_date.is_leap { Some(true) } else { None },
    })
}

#[utoipa::path(
    get,
    path = "/lunar",
    params(LunarQuery),
    responses(
        (status = 200, description = "Convert a solar date to a lunar date", body = DateMonthYear),
        (status = 400, description = "Invalid date format", body = ErrorResponse)
    )
)]
pub async fn get_lunar(Query(q): Query<LunarQuery>) -> Result<Json<DateMonthYear>, AppError> {
    let date = NaiveDate::from_ymd_opt(q.yyyy, q.mm, q.dd)
        .ok_or_else(|| AppError::BadRequest(format!("Invalid solar date: {:02}/{:02}/{}", q.dd, q.mm, q.yyyy)))?;
    
    let tz = q.time_zone.unwrap_or(7.0);
    let lunar_date = to_lunar(date, tz);
    Ok(Json(DateMonthYear {
        dd: lunar_date.day,
        mm: lunar_date.month,
        yyyy: lunar_date.year,
        leap: if lunar_date.is_leap { Some(true) } else { None },
    }))
}

#[utoipa::path(
    get,
    path = "/lunar/{ddMMyyyy}",
    params(
        ("ddMMyyyy" = String, Path, description = "Date formatted as 8 digits (e.g. 12092015)")
    ),
    responses(
        (status = 200, description = "Convert path formatted solar date to lunar date", body = DateMonthYear),
        (status = 400, description = "Invalid date format", body = ErrorResponse)
    )
)]
pub async fn get_lunar_by_path(Path(param): Path<String>) -> Result<Json<DateMonthYear>, AppError> {
    if param.len() != 8 {
        return Err(AppError::BadRequest("Expected 8 digits format: ddMMyyyy".to_string()));
    }
    let dd: u32 = param[0..2].parse().map_err(|_| AppError::BadRequest("Invalid day digits".to_string()))?;
    let mm: u32 = param[2..4].parse().map_err(|_| AppError::BadRequest("Invalid month digits".to_string()))?;
    let yyyy: i32 = param[4..8].parse().map_err(|_| AppError::BadRequest("Invalid year digits".to_string()))?;
    
    let date = NaiveDate::from_ymd_opt(yyyy, mm, dd)
        .ok_or_else(|| AppError::BadRequest(format!("Invalid solar date: {:02}/{:02}/{}", dd, mm, yyyy)))?;
    let lunar_date = to_lunar(date, 7.0);
    Ok(Json(DateMonthYear {
        dd: lunar_date.day,
        mm: lunar_date.month,
        yyyy: lunar_date.year,
        leap: if lunar_date.is_leap { Some(true) } else { None },
    }))
}

#[utoipa::path(
    get,
    path = "/vietnam-holiday",
    params(HolidayQuery),
    responses(
        (status = 200, description = "Check if a date is a Vietnam holiday", body = bool),
        (status = 400, description = "Invalid date format", body = ErrorResponse)
    )
)]
pub async fn check_vietnam_holiday(Query(q): Query<HolidayQuery>) -> Result<Json<bool>, AppError> {
    let date = NaiveDate::from_ymd_opt(q.yyyy, q.mm, q.dd)
        .ok_or_else(|| AppError::BadRequest(format!("Invalid date: {:02}/{:02}/{}", q.dd, q.mm, q.yyyy)))?;
        
    Ok(Json(is_vietnam_holiday(date, 7.0)))
}

