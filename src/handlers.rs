use axum::{extract::Query, Json};
use chrono::{Datelike, Local, NaiveDate};
use serde::Deserialize;

use crate::models::DateMonthYear;
use crate::services::{is_vietnam_holiday, to_lunar};

use utoipa::IntoParams;

#[derive(Deserialize, IntoParams)]
#[allow(non_snake_case)]
pub struct LunarQuery {
    pub dd: String,
    pub MM: String,
    pub yyyy: String,
    pub timeZone: Option<String>,
}

#[derive(Deserialize, IntoParams)]
#[allow(non_snake_case)]
pub struct HolidayQuery {
    pub dd: String,
    pub MM: String,
    pub yyyy: String,
}

#[utoipa::path(
    get,
    path = "/",
    responses(
        (status = 200, description = "Get today's lunar date", body = DateMonthYear)
    )
)]
pub async fn home() -> Json<DateMonthYear> {
    let now = Local::now().naive_local().date();
    let lunar_date = to_lunar(now, 7.0);
    Json(DateMonthYear {
        dd: lunar_date.day() as i32,
        MM: lunar_date.month() as i32,
        yyyy: lunar_date.year() as i32,
    })
}

#[utoipa::path(
    get,
    path = "/lunar",
    params(LunarQuery),
    responses(
        (status = 200, description = "Convert a solar date to a lunar date", body = DateMonthYear),
        (status = 400, description = "Invalid date format", body = String)
    )
)]
pub async fn get_lunar(Query(q): Query<LunarQuery>) -> Result<Json<DateMonthYear>, String> {
    let dd: u32 = q.dd.parse().map_err(|_| "Invalid dd".to_string())?;
    let mm: u32 = q.MM.parse().map_err(|_| "Invalid MM".to_string())?;
    let yyyy: i32 = q.yyyy.parse().map_err(|_| "Invalid yyyy".to_string())?;
    
    let date = NaiveDate::from_ymd_opt(yyyy, mm, dd)
        .ok_or_else(|| "Invalid date".to_string())?;
    
    let tz = q.timeZone.and_then(|t| t.parse().ok()).unwrap_or(7.0);
    let lunar_date = to_lunar(date, tz);
    Ok(Json(DateMonthYear {
        dd: lunar_date.day() as i32,
        MM: lunar_date.month() as i32,
        yyyy: lunar_date.year() as i32,
    }))
}

#[utoipa::path(
    get,
    path = "/vietnam-holiday",
    params(HolidayQuery),
    responses(
        (status = 200, description = "Check if a date is a Vietnam holiday", body = bool),
        (status = 400, description = "Invalid date format", body = String)
    )
)]
pub async fn check_vietnam_holiday(Query(q): Query<HolidayQuery>) -> Result<Json<bool>, String> {
    let dd: u32 = q.dd.parse().map_err(|_| "Invalid dd".to_string())?;
    let mm: u32 = q.MM.parse().map_err(|_| "Invalid MM".to_string())?;
    let yyyy: i32 = q.yyyy.parse().map_err(|_| "Invalid yyyy".to_string())?;
    
    let date = NaiveDate::from_ymd_opt(yyyy, mm, dd)
        .ok_or_else(|| "Invalid date".to_string())?;
        
    Ok(Json(is_vietnam_holiday(date, 7.0)))
}
