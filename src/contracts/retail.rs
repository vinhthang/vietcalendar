//! Service contract and DTOs for Vietnam lunar retail & seasonal marketing calendars.

use std::sync::Arc;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::contracts::models::{CalendarError, LunarDate};
use crate::contracts::recurrence::{Day30Policy, LeapMonthPolicy};

/// Marketing retail vertical tags stored in `config/retail_events.yml`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RetailCategory {
    /// Cultural & gifting festival peaks (Tết, Mid-Autumn, Lantern Festival).
    Festival,
    /// Devotional, ancestor worship, and Buddhist observance peaks.
    Religious,
    /// High-velocity consumer shopping moments (God of Wealth, Bridal season).
    Shopping,
    /// Food & beverage seasonal peaks (Bánh chưng, Bánh trôi, Bánh trung thu).
    FoodBeverage,
    /// Family gathering & intergenerational gift occasions.
    FamilyGift,
}

/// Declarative retail peak definition parsed strictly from `config/retail_events.yml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetailEventRule {
    /// Unique canonical ASCII event slug.
    pub id: String,
    /// Vietnamese retail peak title.
    pub name_vi: String,
    /// English retail peak title.
    pub name_en: String,
    /// Fixed lunar month (`1..=12`) for annual peaks.
    pub lunar_month: i32,
    /// Target lunar day (`1..=30`).
    pub lunar_day: i32,
    /// Handling when `lunar_day == 30` falls in a 29-day month.
    #[serde(default)]
    pub day30_policy: Day30Policy,
    /// Leap month policy for annual retail peaks.
    #[serde(default)]
    pub leap_month_policy: LeapMonthPolicy,
    /// Associated retail categories (at least one required).
    pub categories: Vec<RetailCategory>,
    /// Days prior to peak to begin inventory & campaign preparation.
    pub prep_lead_days: i64,
    /// Days prior to peak to launch early-bird promotions.
    pub early_bird_lead_days: i64,
    /// Concise Vietnamese merchandising/campaign guidance.
    pub campaign_focus_vi: String,
    /// Concise English merchandising/campaign guidance.
    pub campaign_focus_en: String,
}

/// Validated root YAML document for `config/retail_events.yml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetailCatalog {
    /// Schema revision (currently `1`).
    pub version: u32,
    /// Validated annual retail peak definitions.
    pub events: Vec<RetailEventRule>,
}

/// Marketing timeline milestone dates anchored to the resolved peak date.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct RetailCampaignMilestone {
    /// Date (`YYYY-MM-DD`) to begin inventory sourcing & creative prep (`peak - prep_lead_days`).
    pub prep_start: String,
    /// Date (`YYYY-MM-DD`) to launch early-bird campaign (`peak - early_bird_lead_days`).
    pub early_bird: String,
    /// Exact peak retail Gregorian date (`YYYY-MM-DD`).
    pub peak: String,
}

/// Resolved occurrence of a Vietnam retail or monthly Mùng 1/Rằm retail peak.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct RetailEventOccurrence {
    /// Event identifier (`monthly_mung_1_ram` for algorithmic monthly observances, else YAML `id`).
    pub id: String,
    /// Vietnamese display title.
    pub name_vi: String,
    /// English display title.
    pub name_en: String,
    /// Gregorian peak date `YYYY-MM-DD`.
    pub solar_date: String,
    /// Exact converted Vietnamese lunar date.
    #[schema(value_type = crate::contracts::canchi::LunarDateSchema)]
    pub lunar: LunarDate,
    /// Applicable retail categories.
    pub categories: Vec<RetailCategory>,
    /// Pre-calculated campaign milestones.
    pub milestones: RetailCampaignMilestone,
    /// Vietnamese marketing guidance note.
    pub campaign_focus_vi: String,
    /// English marketing guidance note.
    pub campaign_focus_en: String,
}

/// Annual Vietnam retail calendar response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct RetailYearResponse {
    /// Target Gregorian year.
    pub year: i32,
    /// Applied category filter, if any.
    pub category_filter: Option<RetailCategory>,
    /// Total matched occurrences.
    pub count: usize,
    /// Chronologically sorted retail peaks occurring in `year`.
    pub events: Vec<RetailEventOccurrence>,
}

/// Rolling upcoming retail window response (`<= 366` days).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct RetailUpcomingResponse {
    /// Window start Gregorian date `YYYY-MM-DD`.
    pub from_date: String,
    /// Horizon length in days (`1..=366`).
    pub days: i32,
    /// Applied category filter, if any.
    pub category_filter: Option<RetailCategory>,
    /// Total matched occurrences.
    pub count: usize,
    /// Chronologically sorted upcoming retail peaks.
    pub events: Vec<RetailEventOccurrence>,
}

/// Cordis service contract for hot-reloadable Vietnam retail & seasonal calendars.
pub trait RetailCalendarService: Send + Sync + 'static {
    /// Resolve all Vietnam retail peaks and monthly Mùng 1/Rằm peaks occurring in Gregorian `year`.
    ///
    /// # Errors
    /// Returns [`CalendarError`] when `year` or `time_zone` is out of bounds.
    fn retail_events_for_year(
        &self,
        year: i32,
        category: Option<RetailCategory>,
        time_zone: f64,
    ) -> Result<RetailYearResponse, CalendarError>;

    /// Resolve upcoming retail peaks within `[from_date, from_date + days)` (`1..=366` days).
    ///
    /// # Errors
    /// Returns [`CalendarError`] when `days` is outside `1..=366` or `time_zone` is out of bounds.
    fn upcoming_retail_peaks(
        &self,
        from_date: NaiveDate,
        days: i32,
        category: Option<RetailCategory>,
        time_zone: f64,
    ) -> Result<RetailUpcomingResponse, CalendarError>;

    /// Atomically replace the active retail catalog using wait-free `ArcSwap`.
    fn replace_catalog(&self, catalog: Arc<RetailCatalog>);
}
