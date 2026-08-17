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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_app_error_into_response() {
        let err = AppError::BadRequest("Test bad request".to_string());
        let res = err.into_response();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);

        let err_nf = AppError::NotFound("Test not found".to_string());
        let res_nf = err_nf.into_response();
        assert_eq!(res_nf.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_get_lunar_handler() {
        let q = LunarQuery {
            dd: 12,
            mm: 9,
            yyyy: 2015,
            time_zone: Some(7.0),
        };
        let res = get_lunar(Query(q)).await.unwrap();
        assert_eq!(res.0.dd, 30);
        assert_eq!(res.0.mm, 7);
        assert_eq!(res.0.yyyy, 2015);
    }

    #[tokio::test]
    async fn test_get_lunar_handler_invalid_date() {
        let q = LunarQuery {
            dd: 31,
            mm: 2,
            yyyy: 2024,
            time_zone: None,
        };
        let err = get_lunar(Query(q)).await.unwrap_err();
        match err {
            AppError::BadRequest(msg) => assert!(msg.contains("Invalid solar date")),
            _ => panic!("Expected BadRequest"),
        }
    }

    #[tokio::test]
    async fn test_get_lunar_by_path_valid() {
        let res = get_lunar_by_path(Path("12092015".to_string())).await.unwrap();
        assert_eq!(res.0.dd, 30);
        assert_eq!(res.0.mm, 7);
        assert_eq!(res.0.yyyy, 2015);
    }

    #[tokio::test]
    async fn test_get_lunar_by_path_invalid() {
        // Invalid length (< 8)
        let err1 = get_lunar_by_path(Path("1209".to_string())).await.unwrap_err();
        match err1 {
            AppError::BadRequest(msg) => assert!(msg.contains("Expected 8 digits")),
            _ => panic!("Expected BadRequest"),
        }

        // Invalid month digits
        let err2 = get_lunar_by_path(Path("12992015".to_string())).await.unwrap_err();
        match err2 {
            AppError::BadRequest(msg) => assert!(msg.contains("Invalid solar date")),
            _ => panic!("Expected BadRequest"),
        }
    }

    #[tokio::test]
    async fn test_check_vietnam_holiday_handler() {
        let q = HolidayQuery {
            dd: 30,
            mm: 4,
            yyyy: 2015,
        };
        let res = check_vietnam_holiday(Query(q)).await.unwrap();
        assert!(res.0);
    }
}


