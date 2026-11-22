//! Service contract and DTOs for Vietnamese auspicious date selection (*Trạch Nhật*).

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::contracts::canchi::{BilingualLabel, CanChiBreakdown, CanChiPair};
use crate::contracts::models::{CalendarError, LunarDate};

/// Supported event occasion categories influencing Trạch Nhật scoring weights.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Occasion {
    /// Marriage ceremony / engagement (*Cưới hỏi / Giá thú*).
    Wedding,
    /// Construction groundbreaking (*Động thổ*).
    Groundbreaking,
    /// Business or real-estate contract signing (*Ký hợp đồng*).
    ContractSigning,
    /// Housewarming / moving into a new home (*Nhập trạch*).
    MovingIn,
    /// Business grand opening (*Khai trương*).
    GrandOpening,
    /// Balanced everyday auspicious check (*Tổng quát*).
    General,
}

/// Single 2-hour Chinese/Vietnamese branch window within a day.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct HourFortune {
    /// Earthly Branch index (`0 = Tý .. 11 = Hợi`).
    pub branch_index: u8,
    /// Earthly Branch label.
    pub branch: BilingualLabel,
    /// Exact Stem-Branch Can Chi of this two-hour block (*Ngũ Thử Độn*).
    pub can_chi: CanChiPair,
    /// Standard clock span (e.g. `"23:00-01:00"` for Tý, `"09:00-11:00"` for Tỵ).
    pub time_range: String,
    /// Guardian deity (*Thanh Long*, *Minh Đường*, *Bạch Hổ*, etc.).
    pub deity: BilingualLabel,
    /// `true` when the hour is *Hoàng Đạo* (auspicious), `false` for *Hắc Đạo*.
    pub is_hoang_dao: bool,
}

/// Lunar or seasonal taboo alert detected on a candidate day.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct TabooAlert {
    /// Canonical taboo identifier (`tam_nuong`, `nguyet_ky`, `duong_cong_ky_nhat`, `sat_chu`, `tho_tu`).
    pub id: String,
    /// Vietnamese taboo name.
    pub name_vi: String,
    /// English taboo gloss.
    pub name_en: String,
    /// Impact tier (`"major"` or `"moderate"`).
    pub severity: String,
    /// Concise explanation citing the conflicting day/month rule.
    pub reason: String,
}

/// Person-specific compatibility checks (*Tam Tai*, *Kim Lâu*, *Hoang Ốc*, *Xung Tuổi*).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct AgeConflictCheck {
    /// Gregorian birth year checked.
    pub birth_year: i32,
    /// Birth year Can Chi pair.
    pub year_can_chi: CanChiPair,
    /// Traditional Vietnamese *Tuổi Mụ* (`target_lunar_year - birth_year + 1`).
    pub lunar_age: i32,
    /// `true` when target lunar year belongs to the person's 3-year *Tam Tai* cycle.
    pub tam_tai: bool,
    /// `true` when *Tuổi Mụ % 9* hits `[1, 3, 6, 8]` (*Kim Lâu*).
    pub kim_lau: bool,
    /// Specific *Kim Lâu* sub-category (*Thân*, *Thê*, *Tử*, *Súc*) when `kim_lau == true`.
    pub kim_lau_type: Option<BilingualLabel>,
    /// One of the 6 *Hoang Ốc* stages (*Nhất Cát*, *Nhì Nghi*, *Tam Địa Sát*, *Tứ Tấn Tài*, *Ngũ Thọ Tử*, *Lục Hoang Ốc*).
    pub hoang_oc: BilingualLabel,
    /// `true` when the *Hoang Ốc* stage is unfavorable (*Tam Địa Sát*, *Ngũ Thọ Tử*, *Lục Hoang Ốc*).
    pub hoang_oc_is_bad: bool,
    /// `true` when birth branch clashes directly (*Lục Xung*) with the evaluated day branch.
    pub xung_tuoi_day: bool,
    /// Human-readable advisory bullets summarizing detected conflicts.
    pub notes: Vec<String>,
}

/// Complete Trạch Nhật evaluation for one calendar date.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct TrachNhatDayAnalysis {
    /// Gregorian date `YYYY-MM-DD`.
    pub solar_date: String,
    /// Converted Vietnamese lunar date.
    #[schema(value_type = crate::contracts::canchi::LunarDateSchema)]
    pub lunar: LunarDate,
    /// Underlying Can Chi and active solar term breakdown.
    pub can_chi: CanChiBreakdown,
    /// Target occasion weighted in scoring.
    pub occasion: Occasion,
    /// Ruling day deity of the 12-star Hoàng Đạo/Hắc Đạo cycle.
    pub day_deity: BilingualLabel,
    /// `true` for Hoàng Đạo day, `false` for Hắc Đạo day.
    pub is_hoang_dao_day: bool,
    /// One of the 12 *Kiến Trừ Trực*.
    pub truc: BilingualLabel,
    /// Traditional classification (`"auspicious"`, `"neutral"`, or `"inauspicious"`).
    pub truc_quality: String,
    /// Ruling constellation from the 28 Mansions (*Nhị Thập Bát Tú*).
    pub tu_star: BilingualLabel,
    /// `true` when the 28-Mansion constellation is generally favorable.
    pub tu_is_auspicious: bool,
    /// All 12 two-hour blocks ordered `Tý..Hợi` with `is_hoang_dao` flag.
    pub hours: Vec<HourFortune>,
    /// Active major/moderate lunar taboos on this date.
    pub taboos: Vec<TabooAlert>,
    /// Personal age checks when `owner_birth_year` / `partner_birth_year` were supplied.
    pub age_checks: Vec<AgeConflictCheck>,
    /// Deterministic integer fortune score clamped cleanly in `0..=100`.
    pub score: i32,
    /// Bilingual qualitative tier (`Rất tốt`, `Tốt`, `Trung bình`, `Nên tránh`).
    pub rating: BilingualLabel,
    /// Explicit itemized scoring breakdown lines.
    pub reasons: Vec<String>,
}

/// Bounded range query to rank auspicious dates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct AuspiciousSearchQuery {
    /// Inclusive start date `YYYY-MM-DD`.
    pub start_date: String,
    /// Inclusive end date `YYYY-MM-DD` (span must not exceed `366` days).
    pub end_date: String,
    /// Target occasion.
    pub occasion: Occasion,
    /// Optional primary homeowner/person birth year (e.g. `1990`).
    pub owner_birth_year: Option<i32>,
    /// Optional partner/spouse birth year.
    pub partner_birth_year: Option<i32>,
    /// Optional minimum score threshold (`0..=100`).
    pub min_score: Option<i32>,
    /// Optional maximum number of candidates to return (defaults to `10`, max `50`).
    pub limit: Option<usize>,
    /// Timezone offset in hours from UTC (defaults to `7.0`).
    #[serde(default = "default_vn_tz")]
    pub time_zone: f64,
}

fn default_vn_tz() -> f64 {
    7.0
}

/// Ranked auspicious dates returned for a bounded search window.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct AuspiciousSearchResponse {
    /// Occasion evaluated.
    pub occasion: Occasion,
    /// Search window start date `YYYY-MM-DD`.
    pub start_date: String,
    /// Search window end date `YYYY-MM-DD`.
    pub end_date: String,
    /// Total calendar days evaluated (`<= 366`).
    pub evaluated_days: usize,
    /// Candidates ordered strictly by `(score DESC, solar_date ASC)`.
    pub candidates: Vec<TrachNhatDayAnalysis>,
}

/// Cordis service contract for deterministic Vietnamese Trạch Nhật calculations.
pub trait TrachNhatService: Send + Sync + 'static {
    /// Analyze a single date for a target occasion and up to two birth years.
    ///
    /// # Errors
    /// Returns [`CalendarError`] when any birth year is non-positive or `time_zone` / `date` is invalid.
    fn analyze_date(
        &self,
        date: NaiveDate,
        occasion: Occasion,
        owner_birth_year: Option<i32>,
        partner_birth_year: Option<i32>,
        time_zone: f64,
    ) -> Result<TrachNhatDayAnalysis, CalendarError>;

    /// Search and rank auspicious dates across a bounded range (`<= 366` days).
    ///
    /// # Errors
    /// Returns [`CalendarError`] when dates are malformed, `end < start`, span exceeds `366` days, or `birth_year <= 0`.
    fn find_auspicious_dates(
        &self,
        query: &AuspiciousSearchQuery,
    ) -> Result<AuspiciousSearchResponse, CalendarError>;
}
