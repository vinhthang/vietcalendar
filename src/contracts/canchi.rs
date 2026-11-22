//! Service contract and bilingual DTOs for Vietnamese Can Chi and 24 Solar Terms.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::contracts::models::{CalendarError, LunarDate};

/// Bilingual identifier + Vietnamese (diacritics) + English display pair.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct BilingualLabel {
    /// Canonical ASCII slug identifier.
    pub id: String,
    /// Natural Vietnamese display name with full diacritics.
    pub name_vi: String,
    /// English display name or gloss.
    pub name_en: String,
}

/// OpenAPI schema projection matching [`LunarDate`] wire representation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct LunarDateSchema {
    /// Lunar day (`1..=30`).
    pub day: i32,
    /// Lunar month (`1..=12`).
    pub month: i32,
    /// Lunar year.
    pub year: i32,
    /// Leap month flag (*tháng nhuận*).
    pub is_leap: bool,
}

impl BilingualLabel {
    /// Construct a bilingual label from static or owned strings.
    pub fn new(
        id: impl Into<String>,
        name_vi: impl Into<String>,
        name_en: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name_vi: name_vi.into(),
            name_en: name_en.into(),
        }
    }
}

/// Complete Heavenly Stem and Earthly Branch (*Thiên Can — Địa Chi*) pair with Nạp Âm.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct CanChiPair {
    /// Zero-based Heavenly Stem index (`0 = Giáp .. 9 = Quý`).
    pub stem_index: u8,
    /// Zero-based Earthly Branch index (`0 = Tý .. 11 = Hợi`).
    pub branch_index: u8,
    /// Zero-based sexagenary cycle index (`0 = Giáp Tý .. 59 = Quý Hợi`).
    pub sexagenary_index: u8,
    /// Canonical combined ASCII slug (e.g. `"giap_thin"`).
    pub id: String,
    /// Full Vietnamese Can Chi name (e.g. `"Giáp Thìn"`).
    pub name_vi: String,
    /// English elemental animal designation (e.g. `"Yang Wood Dragon"`).
    pub name_en: String,
    /// Heavenly Stem metadata.
    pub stem: BilingualLabel,
    /// Earthly Branch metadata.
    pub branch: BilingualLabel,
    /// Stem Five Element (*Ngũ Hành* of Thiên Can).
    pub stem_element: BilingualLabel,
    /// 60-cycle Nạp Âm elemental designation.
    pub nap_am: BilingualLabel,
    /// Dominant Nạp Âm Five Element.
    pub nap_am_element: BilingualLabel,
}

/// Astronomical solar term (*Tiết Khí*) instant in a target timezone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct SolarTermInfo {
    /// Chronological index within the Gregorian year (`0..24`).
    pub index: u8,
    /// Canonical ASCII solar term identifier (e.g. `"lap_xuan"`).
    pub id: String,
    /// Vietnamese Tiết Khí name (e.g. `"Lập Xuân"`).
    pub name_vi: String,
    /// English solar term name (e.g. `"Start of Spring"`).
    pub name_en: String,
    /// Exact target apparent ecliptic solar longitude in degrees (`0..360`).
    pub solar_longitude_deg: u16,
    /// Local calendar date `YYYY-MM-DD` at the configured timezone offset.
    pub date: String,
    /// Local timestamp formatted with explicit offset (e.g. `"2024-02-04T15:27:09+07:00"`).
    pub timestamp: String,
    /// Whole Unix timestamp seconds (UTC).
    pub unix_seconds: i64,
    /// `true` for the 12 *Tiết* boundaries starting BaZi months (315°, 345°, 15°, ...).
    pub is_major_tiet: bool,
}

/// Active solar longitude and bounding Tiết Khí terms for a given instant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ActiveSolarTermResult {
    /// Local date formatted as `YYYY-MM-DD`.
    pub date: String,
    /// Hour (`0..=23`) evaluated.
    pub hour: u8,
    /// Minute (`0..=59`) evaluated.
    pub minute: u8,
    /// Timezone offset in hours from UTC.
    pub time_zone: f64,
    /// Instantaneous AA98 solar longitude in degrees `[0.0, 360.0)`.
    pub sun_longitude_deg: f64,
    /// Currently active solar term (most recent passed term).
    pub current_term: SolarTermInfo,
    /// Next upcoming solar term.
    pub next_term: SolarTermInfo,
}

/// Full Can Chi breakdown separating Lunar calendar pillars from Solar-Term BaZi pillars.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct CanChiBreakdown {
    /// Gregorian date formatted as `YYYY-MM-DD`.
    pub solar_date: String,
    /// Local hour (`0..=23`).
    pub hour: u8,
    /// Local minute (`0..=59`).
    pub minute: u8,
    /// Timezone offset in hours from UTC.
    pub time_zone: f64,
    /// Converted Vietnamese lunar date in `time_zone`.
    #[schema(value_type = LunarDateSchema)]
    pub lunar: LunarDate,
    /// Can Chi of the Vietnamese lunar year (resets at Mùng 1 Tết).
    pub lunar_year_can_chi: CanChiPair,
    /// Can Chi of the Vietnamese lunar month (Ngũ Hổ Độn on lunar month).
    pub lunar_month_can_chi: CanChiPair,
    /// Can Chi of the BaZi solar year (resets at exact *Lập Xuân*).
    pub bazi_year_can_chi: CanChiPair,
    /// Can Chi of the BaZi solar month (Ngũ Hổ Độn on 12 *Tiết* boundaries).
    pub bazi_month_can_chi: CanChiPair,
    /// Day Can Chi computed from Julian Day Number (`2000-01-01` = `Mậu Ngọ`).
    pub day_can_chi: CanChiPair,
    /// Hour Can Chi computed by *Ngũ Thử Độn* (with `23:00` Giờ Tý rollover).
    pub hour_can_chi: CanChiPair,
    /// `true` when `hour == 23` shifted day-stem lookup forward to next day's Giờ Tý.
    pub hour_stem_day_shifted: bool,
    /// Currently active Tiết Khí at the queried instant.
    pub current_solar_term: SolarTermInfo,
    /// Upcoming Tiết Khí following the queried instant.
    pub next_solar_term: SolarTermInfo,
}

/// Shared Cordis service contract providing Can Chi and 24 Solar Terms calculations.
pub trait CanChiSolarService: Send + Sync + 'static {
    /// Compute the Sexagenary Can Chi pair for a Gregorian date.
    ///
    /// # Errors
    /// Returns [`CalendarError`] when the date falls outside valid calendar year bounds.
    fn day_can_chi(&self, date: NaiveDate) -> Result<CanChiPair, CalendarError>;

    /// Compute full lunar + solar-term Can Chi pillars and active Tiết Khí for a local timestamp.
    ///
    /// # Errors
    /// Returns [`CalendarError`] on invalid `hour`, `minute`, `time_zone`, or calendar year bounds.
    fn full_can_chi(
        &self,
        date: NaiveDate,
        hour: u8,
        minute: u8,
        time_zone: f64,
    ) -> Result<CanChiBreakdown, CalendarError>;

    /// Compute and cache all 24 solar terms occurring inside Gregorian `year` in `time_zone`.
    ///
    /// # Errors
    /// Returns [`CalendarError`] when `year` or `time_zone` is out of bounds.
    fn solar_terms_for_year(
        &self,
        year: i32,
        time_zone: f64,
    ) -> Result<Vec<SolarTermInfo>, CalendarError>;

    /// Determine the instantaneous solar longitude and bounding solar terms at a local instant.
    ///
    /// # Errors
    /// Returns [`CalendarError`] on invalid `hour`, `minute`, `time_zone`, or calendar year bounds.
    fn active_solar_term(
        &self,
        date: NaiveDate,
        hour: u8,
        minute: u8,
        time_zone: f64,
    ) -> Result<ActiveSolarTermResult, CalendarError>;

    /// Compute Can Chi pair directly from a sexagenary index (`0..60`).
    fn pair_from_index(&self, index: u8) -> CanChiPair;

    /// Compute Can Chi pair for any lunar/astrological year integer.
    fn year_can_chi(&self, year: i32) -> CanChiPair;

    /// Compute Can Chi pair for lunar month (`1..=12`) anchored to `year_stem_index`.
    ///
    /// # Errors
    /// Returns [`CalendarError::InvalidLunarMonth`] when `month` is outside `1..=12`.
    fn month_can_chi(&self, year_stem_index: u8, month: i32) -> Result<CanChiPair, CalendarError>;
}
