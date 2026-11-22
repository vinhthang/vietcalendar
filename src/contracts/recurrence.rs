//! Service contract and DTOs for stateless Vietnamese lunar recurrence and RFC 5545 `.ics` export.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::contracts::canchi::BilingualLabel;
use crate::contracts::models::{CalendarError, LunarDate};

/// Policy for recurring lunar Day 30 when a lunar month is *tháng thiếu* (29 days).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum Day30Policy {
    /// Skip Day 30 altogether on 29-day lunar months.
    Skip,
    /// Map Day 30 to Day 29 (*ngày cuối tháng*) on 29-day lunar months.
    #[default]
    LastDayOfMonth,
}

/// Policy for handling Vietnamese lunar leap months (*tháng nhuận*).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum LeapMonthPolicy {
    /// Include only the regular (non-leap) lunar month.
    #[default]
    PrimaryOnly,
    /// Include both regular and leap lunar months when present.
    IncludeLeap,
    /// Emit occurrences only inside the leap month itself.
    LeapOnly,
}

/// Recurrence cadence for custom lunar rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RecurrenceFrequency {
    /// Trigger every lunar month on `lunar_day`.
    Monthly,
    /// Trigger once per lunar year on `(lunar_day, lunar_month)`.
    Yearly,
}

/// Traditional Vietnamese lunar observance presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RecurrencePreset {
    /// Monthly New Moon & Full Moon (*Mùng 1 & Rằm*: days 1, 15).
    NewAndFullMoon,
    /// 4 vegetarian days per lunar month (*Ăn chay Tứ Trai*: days 1, 14, 15, 30/last day).
    VegetarianFourDays,
    /// 6 vegetarian days per lunar month (*Ăn chay Lục Trai*: days 8, 14, 15, 23, 29, 30).
    VegetarianSixDays,
    /// 10 vegetarian days per lunar month (*Ăn chay Thập Trai*: days 1, 8, 14, 15, 18, 23, 24, 28, 29, 30).
    VegetarianTenDays,
}

/// Stateless lunar recurrence calculation and `.ics` export request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct RecurrenceRequest {
    /// Optional event title for JSON/ICS `SUMMARY`.
    pub title: Option<String>,
    /// Optional event description for ICS `DESCRIPTION`.
    pub description: Option<String>,
    /// Custom cadence (`monthly` or `yearly`). Omit when `preset` is set.
    pub frequency: Option<RecurrenceFrequency>,
    /// Built-in traditional preset. Mutually optional with explicit `frequency`.
    pub preset: Option<RecurrencePreset>,
    /// Lunar day (`1..=30`) required when `preset` is `None`.
    pub lunar_day: Option<i32>,
    /// Lunar month (`1..=12`) required when `frequency == Some(Yearly)`.
    pub lunar_month: Option<i32>,
    /// Inclusive Gregorian window start `YYYY-MM-DD`.
    pub start_date: String,
    /// Inclusive Gregorian window end `YYYY-MM-DD` (max `50` years after `start_date`).
    pub end_date: String,
    /// Handling when `lunar_day == 30` falls in a 29-day month.
    #[serde(default)]
    pub day30_policy: Day30Policy,
    /// Handling when a target lunar month repeats as a leap month.
    #[serde(default)]
    pub leap_month_policy: LeapMonthPolicy,
    /// Reference timezone offset from UTC (defaults to `7.0`).
    #[serde(default = "default_vn_timezone")]
    pub time_zone: f64,
    /// Optional reminder alarm trigger minutes before event start (`VALARM`).
    pub reminder_minutes: Option<u32>,
}

fn default_vn_timezone() -> f64 {
    7.0
}

/// Single resolved Gregorian occurrence derived from a lunar recurrence rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct RecurrenceOccurrence {
    /// Gregorian date `YYYY-MM-DD`.
    pub solar_date: String,
    /// Exact converted Vietnamese lunar date.
    #[schema(value_type = crate::contracts::canchi::LunarDateSchema)]
    pub lunar: LunarDate,
    /// Bilingual event occurrence label.
    pub label: BilingualLabel,
    /// `true` if Day 30 resolved to Day 29 under [`Day30Policy::LastDayOfMonth`].
    pub adjusted_from_day30: bool,
    /// `true` if this occurrence falls in a lunar leap month.
    pub is_leap_month_occurrence: bool,
}

/// Expanded stateless recurrence schedule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct RecurrenceResponse {
    /// Effective event title.
    pub title: String,
    /// Window start date `YYYY-MM-DD`.
    pub start_date: String,
    /// Window end date `YYYY-MM-DD`.
    pub end_date: String,
    /// Applied Day-30 fallback policy.
    pub day30_policy: Day30Policy,
    /// Applied leap-month policy.
    pub leap_month_policy: LeapMonthPolicy,
    /// Total matched occurrences.
    pub count: usize,
    /// Ordered Gregorian occurrences inside `[start_date, end_date]`.
    pub occurrences: Vec<RecurrenceOccurrence>,
}

/// Cordis service contract for stateless Vietnamese lunar recurrence expansion and `.ics` formatting.
pub trait LunarRecurrenceService: Send + Sync + 'static {
    /// Expand lunar recurrence occurrences inside `[start_date, end_date]` (`<= 50` years).
    ///
    /// # Errors
    /// Returns [`CalendarError`] when rule parameters, lunar day/month bounds, or date span (`> 50` years) are invalid.
    fn expand_occurrences(
        &self,
        req: &RecurrenceRequest,
    ) -> Result<RecurrenceResponse, CalendarError>;

    /// Expand occurrences and render an RFC 5545 compliant VCALENDAR payload (`CRLF` + `<= 75`-octet line folding).
    ///
    /// # Errors
    /// Propagates validation errors from [`Self::expand_occurrences`].
    fn export_ics(&self, req: &RecurrenceRequest) -> Result<String, CalendarError>;
}
