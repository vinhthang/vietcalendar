//! Shared domain DTOs, errors, and immutable holiday schedule structures.

use std::fmt;

use chrono::{Datelike, NaiveDate, Weekday};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Wire DTO returned by HTTP REST conversion endpoints, preserving legacy Serde aliases.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, ToSchema)]
pub struct DateMonthYear {
    /// Day of the month.
    pub dd: i32,
    /// Month of the year (accepts `"mm"` or `"MM"` on deserialization).
    #[serde(alias = "mm", alias = "MM")]
    pub mm: i32,
    /// Calendar year.
    pub yyyy: i32,
    /// Present and `true` only when the lunar date falls inside a leap month (*tháng nhuận*).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leap: Option<bool>,
}

/// Dedicated domain representation of a Vietnamese Luni-Solar date.
///
/// Never store lunar dates in `chrono::NaiveDate`, because lunar months can contain 30 days
/// in month 2 (which panics Gregorian validation).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, ToSchema)]
pub struct LunarDate {
    /// Lunar day (`1..=29` or `1..=30`).
    pub day: i32,
    /// Lunar month (`1..=12`).
    pub month: i32,
    /// Lunar year.
    pub year: i32,
    /// Whether this date falls inside a leap month (*tháng nhuận*).
    pub is_leap: bool,
}

/// JSON error payload returned by HTTP REST error responses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    /// Human-readable error message.
    pub error: String,
}

/// Domain errors returned by `CalendarService` and `HolidayService`.
#[derive(Debug, Clone, PartialEq)]
pub enum CalendarError {
    /// Solar date failed Gregorian calendar validation.
    InvalidSolarDate(String),
    /// Lunar month fell outside `1..=12`.
    InvalidLunarMonth(i32),
    /// Lunar day fell outside the valid `1..=max_days` (`29` or `30`) range for the month.
    InvalidLunarDay {
        /// Requested lunar day.
        day: i32,
        /// Requested lunar month.
        month: i32,
        /// Requested lunar year.
        year: i32,
        /// Actual length (`29` or `30`) of the targeted lunar month.
        max_days: u8,
    },
    /// Requested `is_leap_month = true` when that year/month is not a leap month.
    InvalidLeapMonth {
        /// Requested lunar month.
        month: i32,
        /// Requested lunar year.
        year: i32,
    },
    /// Timezone offset was non-finite (`NaN`/`inf`) or outside `[-12.0, 14.0]`.
    InvalidTimezone(f64),
    /// Astronomical or Gregorian range overflow prevented date construction.
    ConversionOutOfRange,
}

impl fmt::Display for CalendarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSolarDate(s) => write!(f, "Invalid solar date: {s}"),
            Self::InvalidLunarMonth(m) => {
                write!(f, "Invalid lunar month {m}: expected value in 1..=12")
            }
            Self::InvalidLunarDay {
                day,
                month,
                year,
                max_days,
            } => write!(
                f,
                "Invalid lunar day {day} for month {month:02}/{year} (month has {max_days} days)"
            ),
            Self::InvalidLeapMonth { month, year } => {
                write!(f, "Month {month:02}/{year} is not a lunar leap month")
            }
            Self::InvalidTimezone(tz) => {
                write!(
                    f,
                    "Invalid timezone offset {tz}: expected finite value in -12.0..=14.0"
                )
            }
            Self::ConversionOutOfRange => {
                write!(f, "Converted date falls outside NaiveDate bounds")
            }
        }
    }
}

impl std::error::Error for CalendarError {}

/// Classification of a configured Vietnam public holiday entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HolidayCategory {
    /// Fixed Gregorian solar holiday (`01/01`, `30/04`, `01/05`, `02/09`).
    FixedSolar,
    /// Lunar calendar holiday (Tết Nguyên Đán, Giỗ Tổ Hùng Vương).
    Lunar,
    /// Compensatory day off (*nghỉ bù*) for a weekend holiday.
    Compensatory,
    /// Government-decreed workday swap bridge day (*ngày nghỉ hoán đổi*).
    Bridge,
}

/// Single named public holiday date within a solar year's schedule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HolidayEntry {
    /// Gregorian solar date (`YYYY-MM-DD`).
    pub date: NaiveDate,
    /// Human-readable holiday name.
    pub name: String,
    /// Holiday classification category.
    pub category: HolidayCategory,
}

/// Immutable, binary-searchable holiday schedule for a single solar year.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YearHolidaySchedule {
    /// Target solar year.
    pub year: i32,
    /// Provenance label (e.g., YAML source path or plugin ID).
    pub source: String,
    /// Whether this year has an explicit schedule (vs. weekends-only fallback).
    pub configured: bool,
    entries: Vec<HolidayEntry>,
    holiday_dates: Vec<NaiveDate>,
    working_weekends: Vec<NaiveDate>,
}

impl YearHolidaySchedule {
    /// Constructs a configured `YearHolidaySchedule` with sorted lookup vectors.
    pub fn new(
        year: i32,
        source: impl Into<String>,
        mut entries: Vec<HolidayEntry>,
        mut working_weekends: Vec<NaiveDate>,
    ) -> Self {
        entries.sort_by_key(|e| e.date);
        let mut holiday_dates: Vec<NaiveDate> = entries.iter().map(|e| e.date).collect();
        holiday_dates.sort_unstable();
        holiday_dates.dedup();
        working_weekends.sort_unstable();
        working_weekends.dedup();

        Self {
            year,
            source: source.into(),
            configured: true,
            entries,
            holiday_dates,
            working_weekends,
        }
    }

    /// Constructs an unconfigured fallback schedule (weekends-only, empty holiday/swap lists).
    pub fn weekends_only(year: i32) -> Self {
        Self {
            year,
            source: "fallback:weekends-only".to_string(),
            configured: false,
            entries: Vec::new(),
            holiday_dates: Vec::new(),
            working_weekends: Vec::new(),
        }
    }

    /// Evaluates whether `date` is a day off under this schedule.
    ///
    /// Precedence:
    /// 1. If `date` appears in `working_weekends` (*làm bù*), returns `false` (overrides `Sat`/`Sun`).
    /// 2. If `date` appears in `holiday_dates`, returns `true`.
    /// 3. Otherwise returns `true` for `Saturday`/`Sunday`, `false` for weekdays.
    pub fn is_holiday_date(&self, date: NaiveDate) -> bool {
        if self.working_weekends.binary_search(&date).is_ok() {
            return false;
        }
        if self.holiday_dates.binary_search(&date).is_ok() {
            return true;
        }
        matches!(date.weekday(), Weekday::Sat | Weekday::Sun)
    }

    /// Returns true if `date` is marked as a swapped working weekend day (*ngày đi làm bù*).
    pub fn is_working_weekend(&self, date: NaiveDate) -> bool {
        self.working_weekends.binary_search(&date).is_ok()
    }

    /// Returns all sorted configured holiday entries for this year.
    pub fn entries(&self) -> &[HolidayEntry] {
        &self.entries
    }

    /// Returns all sorted swapped working weekend dates (*làm bù*) for this year.
    pub fn working_weekends(&self) -> &[NaiveDate] {
        &self.working_weekends
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_date_month_year_serde() {
        let json_data = r#"{"dd": 15, "MM": 8, "yyyy": 2024}"#;
        let parsed: DateMonthYear = serde_json::from_str(json_data).unwrap();
        assert_eq!(
            parsed,
            DateMonthYear {
                dd: 15,
                mm: 8,
                yyyy: 2024,
                leap: None
            }
        );

        let json_data_leap = r#"{"dd": 15, "mm": 8, "yyyy": 2024, "leap": true}"#;
        let parsed_leap: DateMonthYear = serde_json::from_str(json_data_leap).unwrap();
        assert_eq!(parsed_leap.leap, Some(true));

        let serialized = serde_json::to_string(&parsed).unwrap();
        assert!(!serialized.contains("leap"));
    }

    #[test]
    fn test_lunar_date_serde() {
        let lunar = LunarDate {
            day: 1,
            month: 1,
            year: 2024,
            is_leap: false,
        };
        let serialized = serde_json::to_string(&lunar).unwrap();
        let deserialized: LunarDate = serde_json::from_str(&serialized).unwrap();
        assert_eq!(lunar, deserialized);
    }

    #[test]
    fn test_year_holiday_schedule_swapped_weekend() {
        let bridge = NaiveDate::from_ymd_opt(2024, 4, 29).unwrap();
        let swap_sat = NaiveDate::from_ymd_opt(2024, 5, 4).unwrap();
        let sunday = NaiveDate::from_ymd_opt(2024, 5, 5).unwrap();
        let weekday = NaiveDate::from_ymd_opt(2024, 5, 6).unwrap();

        let sched = YearHolidaySchedule::new(
            2024,
            "test",
            vec![HolidayEntry {
                date: bridge,
                name: "Bridge".into(),
                category: HolidayCategory::Bridge,
            }],
            vec![swap_sat],
        );

        assert!(sched.is_holiday_date(bridge));
        assert!(!sched.is_holiday_date(swap_sat));
        assert!(sched.is_working_weekend(swap_sat));
        assert!(sched.is_holiday_date(sunday));
        assert!(!sched.is_holiday_date(weekday));
    }
}
