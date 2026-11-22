//! `HttpHolidaysPlugin`: REST endpoint for data-driven Vietnam public holiday lookups.

use std::sync::Arc;

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use chrono::NaiveDate;
use serde::Deserialize;
use serde_json::Value;
use utoipa::{IntoParams, OpenApi};

use crate::contracts::holidays::HolidayService;
use crate::contracts::http::HttpRouterService;
use crate::contracts::models::ErrorResponse;
use crate::kernel::context::Context;
use crate::kernel::effect::BoxFuture;
use crate::kernel::error::KernelError;
use crate::kernel::fiber::{Plugin, PluginMeta};
use crate::plugins::http_server::{AppError, JsonQuery};

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct HolidayQuery {
    /// Solar day (`1..=31`).
    pub dd: u32,
    /// Solar month (`1..=12`, accepts `mm` or `MM`).
    #[serde(alias = "mm", alias = "MM")]
    pub mm: u32,
    /// Solar year.
    pub yyyy: i32,
    /// Optional timezone offset in hours (accepts `timezone` or `timeZone`, default `7.0`).
    #[serde(alias = "timeZone", alias = "timezone")]
    #[param(rename = "timezone")]
    pub time_zone: Option<f64>,
}

#[derive(OpenApi)]
#[openapi(paths(check_vietnam_holiday), components(schemas(ErrorResponse)))]
struct HolidaysApiDoc;

/// Check whether a solar date is a Vietnam holiday or weekend under configured schedules.
#[utoipa::path(
    get,
    path = "/vietnam-holiday",
    params(HolidayQuery),
    responses(
        (status = 200, description = "Check if a date is a Vietnam holiday", body = bool),
        (status = 400, description = "Invalid date or timezone", body = ErrorResponse)
    )
)]
pub async fn check_vietnam_holiday(
    State(holidays): State<Arc<dyn HolidayService>>,
    JsonQuery(q): JsonQuery<HolidayQuery>,
) -> Result<Json<bool>, AppError> {
    let date = NaiveDate::from_ymd_opt(q.yyyy, q.mm, q.dd).ok_or_else(|| {
        AppError::BadRequest(format!("Invalid date: {:02}/{:02}/{}", q.dd, q.mm, q.yyyy))
    })?;

    let tz = q.time_zone.unwrap_or(7.0);
    let is_holiday = holidays.is_holiday_with_timezone(date, tz)?;
    Ok(Json(is_holiday))
}

/// Cordis plugin mounting `GET /vietnam-holiday` onto `HttpRouterService`.
#[derive(Debug, Default)]
pub struct HttpHolidaysPlugin;

impl Plugin for HttpHolidaysPlugin {
    fn meta(&self) -> PluginMeta {
        PluginMeta::new("http_holidays")
            .requires::<dyn HolidayService>()
            .requires::<dyn HttpRouterService>()
    }

    fn mount<'a>(&'a self, ctx: Context, _config: Value) -> BoxFuture<'a, Result<(), KernelError>> {
        Box::pin(async move {
            let holidays = ctx.inject::<dyn HolidayService>()?;
            let router_svc = ctx.inject::<dyn HttpRouterService>()?;

            let router = Router::new()
                .route("/vietnam-holiday", get(check_vietnam_holiday))
                .with_state(holidays);

            let disposer = router_svc.mount_routes(router, Some(HolidaysApiDoc::openapi()));
            ctx.on_dispose_sync(disposer);
            Ok(())
        })
    }
}
