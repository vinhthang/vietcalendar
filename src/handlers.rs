use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::{Datelike, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::models::DateMonthYear;
use crate::services::{is_vietnam_holiday, to_lunar, to_solar};

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

#[derive(Deserialize, IntoParams)]
#[allow(non_snake_case)]
pub struct ConvertSolarToLunarQuery {
    #[serde(alias = "timeZone", alias = "timezone")]
    pub time_zone: Option<f64>,
}

#[derive(Deserialize, IntoParams)]
#[allow(non_snake_case)]
pub struct ConvertLunarToSolarQuery {
    pub leap: Option<bool>,
    #[serde(alias = "timeZone", alias = "timezone")]
    pub time_zone: Option<f64>,
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
        (status = 200, description = "Convert a solar date to a lunar date via query params", body = DateMonthYear),
        (status = 400, description = "Invalid date format", body = ErrorResponse)
    )
)]
pub async fn get_lunar(Query(q): Query<LunarQuery>) -> Result<Json<DateMonthYear>, AppError> {
    let date = NaiveDate::from_ymd_opt(q.yyyy, q.mm, q.dd).ok_or_else(|| {
        AppError::BadRequest(format!(
            "Invalid solar date: {:02}/{:02}/{}",
            q.dd, q.mm, q.yyyy
        ))
    })?;

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
    Path(date_str): Path<String>,
    Query(q): Query<ConvertSolarToLunarQuery>,
) -> Result<Json<DateMonthYear>, AppError> {
    let date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").map_err(|_| {
        AppError::BadRequest("Expected ISO date format: YYYY-MM-DD (e.g. 2015-09-12)".to_string())
    })?;

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
    Path(date_str): Path<String>,
    Query(q): Query<ConvertLunarToSolarQuery>,
) -> Result<Json<DateMonthYear>, AppError> {
    let parts: Vec<&str> = date_str.split('-').collect();
    if parts.len() != 3 {
        return Err(AppError::BadRequest(
            "Expected format: YYYY-MM-DD (e.g. 2015-07-30)".to_string(),
        ));
    }
    let yyyy: i32 = parts[0]
        .parse()
        .map_err(|_| AppError::BadRequest("Invalid year".to_string()))?;
    let mm: i32 = parts[1]
        .parse()
        .map_err(|_| AppError::BadRequest("Invalid month".to_string()))?;
    let dd: i32 = parts[2]
        .parse()
        .map_err(|_| AppError::BadRequest("Invalid day".to_string()))?;

    if !(1..=12).contains(&mm) || !(1..=30).contains(&dd) {
        return Err(AppError::BadRequest(format!(
            "Invalid lunar date range: {:02}/{:02}/{}",
            dd, mm, yyyy
        )));
    }

    let tz = q.time_zone.unwrap_or(7.0);
    let is_leap = q.leap.unwrap_or(false);
    let solar_date = to_solar(dd, mm, yyyy, is_leap, tz).ok_or_else(|| {
        AppError::BadRequest(format!(
            "Invalid lunar date or leap month: {:02}/{:02}/{} (leap: {})",
            dd, mm, yyyy, is_leap
        ))
    })?;

    Ok(Json(DateMonthYear {
        dd: solar_date.day() as i32,
        mm: solar_date.month() as i32,
        yyyy: solar_date.year(),
        leap: None,
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
    let date = NaiveDate::from_ymd_opt(q.yyyy, q.mm, q.dd).ok_or_else(|| {
        AppError::BadRequest(format!("Invalid date: {:02}/{:02}/{}", q.dd, q.mm, q.yyyy))
    })?;

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
    async fn test_get_solar_to_lunar_endpoint() {
        let q = ConvertSolarToLunarQuery { time_zone: None };
        let res = get_solar_to_lunar(Path("2015-09-12".to_string()), Query(q))
            .await
            .unwrap();
        assert_eq!(res.0.dd, 30);
        assert_eq!(res.0.mm, 7);
        assert_eq!(res.0.yyyy, 2015);

        // Invalid ISO format
        let err = get_solar_to_lunar(
            Path("12-09-2015".to_string()),
            Query(ConvertSolarToLunarQuery { time_zone: None }),
        )
        .await
        .unwrap_err();
        match err {
            AppError::BadRequest(msg) => assert!(msg.contains("Expected ISO date format")),
            _ => panic!("Expected BadRequest"),
        }
    }

    #[tokio::test]
    async fn test_get_lunar_to_solar_endpoint() {
        let q = ConvertLunarToSolarQuery {
            leap: Some(false),
            time_zone: None,
        };
        let res = get_lunar_to_solar(Path("2015-07-30".to_string()), Query(q))
            .await
            .unwrap();
        assert_eq!(res.0.dd, 12);
        assert_eq!(res.0.mm, 9);
        assert_eq!(res.0.yyyy, 2015);

        // Leap month test: 2004 month 2 leap
        let q_leap = ConvertLunarToSolarQuery {
            leap: Some(true),
            time_zone: None,
        };
        let res_leap = get_lunar_to_solar(Path("2004-02-11".to_string()), Query(q_leap))
            .await
            .unwrap();
        assert_eq!(res_leap.0.dd, 31);
        assert_eq!(res_leap.0.mm, 3);
        assert_eq!(res_leap.0.yyyy, 2004);

        // Invalid format
        let err = get_lunar_to_solar(
            Path("20150730".to_string()),
            Query(ConvertLunarToSolarQuery {
                leap: None,
                time_zone: None,
            }),
        )
        .await
        .unwrap_err();
        match err {
            AppError::BadRequest(msg) => assert!(msg.contains("Expected format")),
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
