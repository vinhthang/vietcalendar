//! Thin HTTP REST & OpenAPI Cordis plugins exposing all Vietnamese Lunar niche services.

use std::sync::Arc;

use axum::extract::{FromRequest, Path, Request, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{Duration, NaiveDate, Utc};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::Value;
use utoipa::{IntoParams, OpenApi};

use crate::contracts::astrology::{
    AstrologyChartResponse, AstrologyQuery, AstrologyService, ChartSystem, Gender,
};
use crate::contracts::canchi::{CanChiBreakdown, CanChiSolarService, SolarTermInfo};
use crate::contracts::http::HttpRouterService;
use crate::contracts::models::{CalendarError, ErrorResponse};
use crate::contracts::recurrence::{LunarRecurrenceService, RecurrenceRequest, RecurrenceResponse};
use crate::contracts::retail::{
    RetailCalendarService, RetailCategory, RetailUpcomingResponse, RetailYearResponse,
};
use crate::contracts::trach_nhat::{
    AuspiciousSearchQuery, AuspiciousSearchResponse, Occasion, TrachNhatDayAnalysis,
    TrachNhatService,
};
use crate::kernel::context::Context;
use crate::kernel::effect::BoxFuture;
use crate::kernel::error::KernelError;
use crate::kernel::fiber::{Plugin, PluginMeta};
use crate::plugins::http_calendar::parse_ymd_ascii;
use crate::plugins::http_server::{AppError, JsonQuery};

/// Custom Axum JSON body extractor mapping deserialization errors into `HTTP 400` `ErrorResponse`.
#[derive(Debug, Clone)]
pub struct AppJson<T>(pub T);

impl<S, T> FromRequest<S> for AppJson<T>
where
    S: Send + Sync,
    T: DeserializeOwned,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        Json::<T>::from_request(req, state)
            .await
            .map(|Json(val)| Self(val))
            .map_err(|err| AppError::BadRequest(err.body_text()))
    }
}

fn parse_solar_date(raw: &str) -> Result<NaiveDate, AppError> {
    let (y, m, d) = parse_ymd_ascii(raw).ok_or_else(|| {
        AppError::BadRequest(format!("invalid date '{raw}'; expected YYYY-MM-DD"))
    })?;
    let month_u = u32::try_from(m).map_err(|_| AppError::BadRequest(raw.to_string()))?;
    let day_u = u32::try_from(d).map_err(|_| AppError::BadRequest(raw.to_string()))?;
    NaiveDate::from_ymd_opt(y, month_u, day_u).ok_or_else(|| {
        AppError::BadRequest(CalendarError::InvalidSolarDate(raw.to_string()).to_string())
    })
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct CanChiDateQuery {
    /// Local hour (`0..=23`, default `12`).
    pub hour: Option<u8>,
    /// Local minute (`0..=59`, default `0`).
    pub minute: Option<u8>,
    /// Timezone offset in hours from UTC (default `7.0`).
    #[serde(alias = "timeZone", alias = "timezone")]
    #[param(rename = "timezone")]
    pub time_zone: Option<f64>,
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct TimezoneOnlyQuery {
    /// Timezone offset in hours from UTC (default `7.0`).
    #[serde(alias = "timeZone", alias = "timezone")]
    #[param(rename = "timezone")]
    pub time_zone: Option<f64>,
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct BaziDateQuery {
    /// Birth hour (`0..=23`, default `12`).
    pub hour: Option<u8>,
    /// Birth minute (`0..=59`, default `0`).
    pub minute: Option<u8>,
    /// Gender polarity (`male` or `female`, default `male`).
    pub gender: Option<Gender>,
    /// Timezone offset in hours from UTC (default `7.0`).
    #[serde(alias = "timeZone", alias = "timezone")]
    #[param(rename = "timezone")]
    pub time_zone: Option<f64>,
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct TrachNhatDayQuery {
    /// Event occasion category (default `general`).
    pub occasion: Option<Occasion>,
    /// Optional primary owner/subject birth year.
    pub owner_birth_year: Option<i32>,
    /// Optional partner birth year.
    pub partner_birth_year: Option<i32>,
    /// Timezone offset in hours from UTC (default `7.0`).
    #[serde(alias = "timeZone", alias = "timezone")]
    #[param(rename = "timezone")]
    pub time_zone: Option<f64>,
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct RetailYearQuery {
    /// Optional retail category filter (`festival`, `religious`, `shopping`, `food_beverage`, `family_gift`).
    pub category: Option<RetailCategory>,
    /// Timezone offset in hours from UTC (default `7.0`).
    #[serde(alias = "timeZone", alias = "timezone")]
    #[param(rename = "timezone")]
    pub time_zone: Option<f64>,
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct RetailUpcomingQuery {
    /// Window start date `YYYY-MM-DD` (defaults to today UTC+7).
    pub from: Option<String>,
    /// Lookahead horizon in days (`1..=366`, default `60`).
    pub days: Option<i32>,
    /// Optional retail category filter.
    pub category: Option<RetailCategory>,
    /// Timezone offset in hours from UTC (default `7.0`).
    #[serde(alias = "timeZone", alias = "timezone")]
    #[param(rename = "timezone")]
    pub time_zone: Option<f64>,
}

// ---------- 1. HttpCanChiPlugin ----------

#[derive(OpenApi)]
#[openapi(paths(get_canchi_by_date, get_solar_terms_by_year))]
struct CanChiApiDoc;

/// Compute full Can Chi pillars and active Tiết Khí for a Gregorian date.
#[utoipa::path(
    get,
    path = "/canchi/{date}",
    params(
        ("date" = String, Path, description = "Gregorian date YYYY-MM-DD"),
        CanChiDateQuery
    ),
    responses(
        (status = 200, description = "Can Chi & Tiết Khí breakdown", body = CanChiBreakdown),
        (status = 400, description = "Invalid date, hour, minute, or timezone", body = ErrorResponse)
    )
)]
pub async fn get_canchi_by_date(
    State(svc): State<Arc<dyn CanChiSolarService>>,
    Path(date_str): Path<String>,
    JsonQuery(q): JsonQuery<CanChiDateQuery>,
) -> Result<Json<CanChiBreakdown>, AppError> {
    let date = parse_solar_date(&date_str)?;
    let breakdown = svc.full_can_chi(
        date,
        q.hour.unwrap_or(12),
        q.minute.unwrap_or(0),
        q.time_zone.unwrap_or(7.0),
    )?;
    Ok(Json(breakdown))
}

/// List all 24 astronomical solar terms (*Tiết Khí*) for a Gregorian year.
#[utoipa::path(
    get,
    path = "/solar-terms/{year}",
    params(
        ("year" = i32, Path, description = "Gregorian year (e.g. 2024)"),
        TimezoneOnlyQuery
    ),
    responses(
        (status = 200, description = "24 Solar Terms ordered chronologically", body = Vec<SolarTermInfo>),
        (status = 400, description = "Invalid year or timezone", body = ErrorResponse)
    )
)]
pub async fn get_solar_terms_by_year(
    State(svc): State<Arc<dyn CanChiSolarService>>,
    Path(year): Path<i32>,
    JsonQuery(q): JsonQuery<TimezoneOnlyQuery>,
) -> Result<Json<Vec<SolarTermInfo>>, AppError> {
    let terms = svc.solar_terms_for_year(year, q.time_zone.unwrap_or(7.0))?;
    Ok(Json(terms))
}

/// Cordis HTTP plugin mounting Can Chi and 24 Solar Terms REST endpoints.
#[derive(Debug, Default)]
pub struct HttpCanChiPlugin;

impl Plugin for HttpCanChiPlugin {
    fn meta(&self) -> PluginMeta {
        PluginMeta::new("http_canchi")
            .requires::<dyn CanChiSolarService>()
            .requires::<dyn HttpRouterService>()
    }

    fn mount<'a>(&'a self, ctx: Context, _config: Value) -> BoxFuture<'a, Result<(), KernelError>> {
        Box::pin(async move {
            let svc = ctx.inject::<dyn CanChiSolarService>()?;
            let router_svc = ctx.inject::<dyn HttpRouterService>()?;
            let router = Router::new()
                .route("/canchi/{date}", get(get_canchi_by_date))
                .route("/solar-terms/{year}", get(get_solar_terms_by_year))
                .with_state(svc);
            let disposer = router_svc.mount_routes(router, Some(CanChiApiDoc::openapi()));
            ctx.on_dispose_sync(disposer);
            Ok(())
        })
    }
}

// ---------- 2. HttpAstrologyPlugin ----------

#[derive(OpenApi)]
#[openapi(paths(post_astrology_chart, get_bazi_by_date))]
struct AstrologyApiDoc;

/// Generate deterministic Vietnamese Tử Vi and/or BaZi birth chart.
#[utoipa::path(
    post,
    path = "/astrology/chart",
    request_body = AstrologyQuery,
    responses(
        (status = 200, description = "Generated BaZi and/or Tử Vi chart", body = AstrologyChartResponse),
        (status = 400, description = "Invalid birth parameters", body = ErrorResponse)
    )
)]
pub async fn post_astrology_chart(
    State(svc): State<Arc<dyn AstrologyService>>,
    AppJson(query): AppJson<AstrologyQuery>,
) -> Result<Json<AstrologyChartResponse>, AppError> {
    Ok(Json(svc.generate_chart(&query)?))
}

/// Quick GET convenience endpoint for a BaZi Four Pillars chart by Gregorian birth date.
#[utoipa::path(
    get,
    path = "/astrology/bazi/{date}",
    params(
        ("date" = String, Path, description = "Gregorian birth date YYYY-MM-DD"),
        BaziDateQuery
    ),
    responses(
        (status = 200, description = "Generated BaZi chart", body = AstrologyChartResponse),
        (status = 400, description = "Invalid birth parameters", body = ErrorResponse)
    )
)]
pub async fn get_bazi_by_date(
    State(svc): State<Arc<dyn AstrologyService>>,
    Path(date): Path<String>,
    JsonQuery(q): JsonQuery<BaziDateQuery>,
) -> Result<Json<AstrologyChartResponse>, AppError> {
    let query = AstrologyQuery {
        date,
        hour: q.hour.unwrap_or(12),
        minute: q.minute.unwrap_or(0),
        gender: q.gender.unwrap_or(Gender::Male),
        system: ChartSystem::Bazi,
        is_lunar: false,
        is_leap_month: false,
        time_zone: q.time_zone.unwrap_or(7.0),
    };
    Ok(Json(svc.generate_chart(&query)?))
}

/// Cordis HTTP plugin mounting BaZi and Tử Vi REST endpoints.
#[derive(Debug, Default)]
pub struct HttpAstrologyPlugin;

impl Plugin for HttpAstrologyPlugin {
    fn meta(&self) -> PluginMeta {
        PluginMeta::new("http_astrology")
            .requires::<dyn AstrologyService>()
            .requires::<dyn HttpRouterService>()
    }

    fn mount<'a>(&'a self, ctx: Context, _config: Value) -> BoxFuture<'a, Result<(), KernelError>> {
        Box::pin(async move {
            let svc = ctx.inject::<dyn AstrologyService>()?;
            let router_svc = ctx.inject::<dyn HttpRouterService>()?;
            let router = Router::new()
                .route("/astrology/chart", post(post_astrology_chart))
                .route("/astrology/bazi/{date}", get(get_bazi_by_date))
                .with_state(svc);
            let disposer = router_svc.mount_routes(router, Some(AstrologyApiDoc::openapi()));
            ctx.on_dispose_sync(disposer);
            Ok(())
        })
    }
}

// ---------- 3. HttpTrachNhatPlugin ----------

#[derive(OpenApi)]
#[openapi(paths(get_trach_nhat_day, post_trach_nhat_search))]
struct TrachNhatApiDoc;

/// Evaluate auspiciousness, Hoàng Đạo hours, 12 Trực, 28 Tú, taboos & age clashes for a single date.
#[utoipa::path(
    get,
    path = "/trach-nhat/day/{date}",
    params(
        ("date" = String, Path, description = "Gregorian date YYYY-MM-DD"),
        TrachNhatDayQuery
    ),
    responses(
        (status = 200, description = "Complete Trạch Nhật day evaluation", body = TrachNhatDayAnalysis),
        (status = 400, description = "Invalid date, occasion, birth year, or timezone", body = ErrorResponse)
    )
)]
pub async fn get_trach_nhat_day(
    State(svc): State<Arc<dyn TrachNhatService>>,
    Path(date_str): Path<String>,
    JsonQuery(q): JsonQuery<TrachNhatDayQuery>,
) -> Result<Json<TrachNhatDayAnalysis>, AppError> {
    let date = parse_solar_date(&date_str)?;
    let res = svc.analyze_date(
        date,
        q.occasion.unwrap_or(Occasion::General),
        q.owner_birth_year,
        q.partner_birth_year,
        q.time_zone.unwrap_or(7.0),
    )?;
    Ok(Json(res))
}

/// Search and rank auspicious dates over a bounded horizon (`<= 366` days).
#[utoipa::path(
    post,
    path = "/trach-nhat/search",
    request_body = AuspiciousSearchQuery,
    responses(
        (status = 200, description = "Ranked auspicious dates", body = AuspiciousSearchResponse),
        (status = 400, description = "Invalid search range or parameters", body = ErrorResponse)
    )
)]
pub async fn post_trach_nhat_search(
    State(svc): State<Arc<dyn TrachNhatService>>,
    AppJson(query): AppJson<AuspiciousSearchQuery>,
) -> Result<Json<AuspiciousSearchResponse>, AppError> {
    Ok(Json(svc.find_auspicious_dates(&query)?))
}

/// Cordis HTTP plugin mounting Trạch Nhật auspicious date REST endpoints.
#[derive(Debug, Default)]
pub struct HttpTrachNhatPlugin;

impl Plugin for HttpTrachNhatPlugin {
    fn meta(&self) -> PluginMeta {
        PluginMeta::new("http_trach_nhat")
            .requires::<dyn TrachNhatService>()
            .requires::<dyn HttpRouterService>()
    }

    fn mount<'a>(&'a self, ctx: Context, _config: Value) -> BoxFuture<'a, Result<(), KernelError>> {
        Box::pin(async move {
            let svc = ctx.inject::<dyn TrachNhatService>()?;
            let router_svc = ctx.inject::<dyn HttpRouterService>()?;
            let router = Router::new()
                .route("/trach-nhat/day/{date}", get(get_trach_nhat_day))
                .route("/trach-nhat/search", post(post_trach_nhat_search))
                .with_state(svc);
            let disposer = router_svc.mount_routes(router, Some(TrachNhatApiDoc::openapi()));
            ctx.on_dispose_sync(disposer);
            Ok(())
        })
    }
}

// ---------- 4. HttpRecurrencePlugin ----------

#[derive(OpenApi)]
#[openapi(paths(post_recurrence_expand, post_recurrence_ics))]
struct RecurrenceApiDoc;

/// Expand stateless lunar recurrence occurrences across a window (`<= 50` years).
#[utoipa::path(
    post,
    path = "/recurrence/expand",
    request_body = RecurrenceRequest,
    responses(
        (status = 200, description = "Expanded Gregorian occurrences", body = RecurrenceResponse),
        (status = 400, description = "Invalid recurrence parameters", body = ErrorResponse)
    )
)]
pub async fn post_recurrence_expand(
    State(svc): State<Arc<dyn LunarRecurrenceService>>,
    AppJson(req): AppJson<RecurrenceRequest>,
) -> Result<Json<RecurrenceResponse>, AppError> {
    Ok(Json(svc.expand_occurrences(&req)?))
}

/// Export an RFC 5545 `.ics` VCALENDAR feed (`CRLF` + `<= 75`-octet UTF-8 folding).
#[utoipa::path(
    post,
    path = "/recurrence/ics",
    request_body = RecurrenceRequest,
    responses(
        (status = 200, description = "RFC 5545 iCalendar payload", content_type = "text/calendar", body = String),
        (status = 400, description = "Invalid recurrence parameters", body = ErrorResponse)
    )
)]
pub async fn post_recurrence_ics(
    State(svc): State<Arc<dyn LunarRecurrenceService>>,
    AppJson(req): AppJson<RecurrenceRequest>,
) -> Result<Response, AppError> {
    let ics = svc.export_ics(&req)?;
    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/calendar; charset=utf-8")],
        ics,
    )
        .into_response())
}

/// Cordis HTTP plugin mounting stateless lunar recurrence REST endpoints.
#[derive(Debug, Default)]
pub struct HttpRecurrencePlugin;

impl Plugin for HttpRecurrencePlugin {
    fn meta(&self) -> PluginMeta {
        PluginMeta::new("http_recurrence")
            .requires::<dyn LunarRecurrenceService>()
            .requires::<dyn HttpRouterService>()
    }

    fn mount<'a>(&'a self, ctx: Context, _config: Value) -> BoxFuture<'a, Result<(), KernelError>> {
        Box::pin(async move {
            let svc = ctx.inject::<dyn LunarRecurrenceService>()?;
            let router_svc = ctx.inject::<dyn HttpRouterService>()?;
            let router = Router::new()
                .route("/recurrence/expand", post(post_recurrence_expand))
                .route("/recurrence/ics", post(post_recurrence_ics))
                .with_state(svc);
            let disposer = router_svc.mount_routes(router, Some(RecurrenceApiDoc::openapi()));
            ctx.on_dispose_sync(disposer);
            Ok(())
        })
    }
}

// ---------- 5. HttpRetailPlugin ----------

#[derive(OpenApi)]
#[openapi(paths(get_retail_year, get_retail_upcoming))]
struct RetailApiDoc;

/// Resolve Vietnam retail & marketing peaks for a Gregorian year.
#[utoipa::path(
    get,
    path = "/retail/year/{year}",
    params(
        ("year" = i32, Path, description = "Target Gregorian year"),
        RetailYearQuery
    ),
    responses(
        (status = 200, description = "Annual Vietnam retail & cultural calendar", body = RetailYearResponse),
        (status = 400, description = "Invalid year, category, or timezone", body = ErrorResponse)
    )
)]
pub async fn get_retail_year(
    State(svc): State<Arc<dyn RetailCalendarService>>,
    Path(year): Path<i32>,
    JsonQuery(q): JsonQuery<RetailYearQuery>,
) -> Result<Json<RetailYearResponse>, AppError> {
    Ok(Json(svc.retail_events_for_year(
        year,
        q.category,
        q.time_zone.unwrap_or(7.0),
    )?))
}

/// List upcoming Vietnam retail & marketing peaks within `[from, from + days)` (`1..=366` days).
#[utoipa::path(
    get,
    path = "/retail/upcoming",
    params(RetailUpcomingQuery),
    responses(
        (status = 200, description = "Upcoming retail peaks with milestones", body = RetailUpcomingResponse),
        (status = 400, description = "Invalid date, horizon, category, or timezone", body = ErrorResponse)
    )
)]
pub async fn get_retail_upcoming(
    State(svc): State<Arc<dyn RetailCalendarService>>,
    JsonQuery(q): JsonQuery<RetailUpcomingQuery>,
) -> Result<Json<RetailUpcomingResponse>, AppError> {
    let from = match q.from {
        Some(ref s) => parse_solar_date(s)?,
        None => (Utc::now() + Duration::hours(7)).date_naive(),
    };
    Ok(Json(svc.upcoming_retail_peaks(
        from,
        q.days.unwrap_or(60),
        q.category,
        q.time_zone.unwrap_or(7.0),
    )?))
}

/// Cordis HTTP plugin mounting Vietnam retail calendar REST endpoints.
#[derive(Debug, Default)]
pub struct HttpRetailPlugin;

impl Plugin for HttpRetailPlugin {
    fn meta(&self) -> PluginMeta {
        PluginMeta::new("http_retail")
            .requires::<dyn RetailCalendarService>()
            .requires::<dyn HttpRouterService>()
    }

    fn mount<'a>(&'a self, ctx: Context, _config: Value) -> BoxFuture<'a, Result<(), KernelError>> {
        Box::pin(async move {
            let svc = ctx.inject::<dyn RetailCalendarService>()?;
            let router_svc = ctx.inject::<dyn HttpRouterService>()?;
            let router = Router::new()
                .route("/retail/year/{year}", get(get_retail_year))
                .route("/retail/upcoming", get(get_retail_upcoming))
                .with_state(svc);
            let disposer = router_svc.mount_routes(router, Some(RetailApiDoc::openapi()));
            ctx.on_dispose_sync(disposer);
            Ok(())
        })
    }
}
