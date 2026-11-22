//! Service contract (`CalendarService`) for Vietnamese Luni-Solar conversions.

use chrono::NaiveDate;

use crate::contracts::models::{CalendarError, LunarDate};

/// Abstract astronomical calendar capability provided by `VietnamCalendarPlugin`.
pub trait CalendarService: Send + Sync + 'static {
    /// Converts a Gregorian `date` in `time_zone` hours (e.g. `7.0` for Vietnam) to `LunarDate`.
    ///
    /// # Errors
    /// Returns `CalendarError::InvalidTimezone` if `time_zone` is non-finite or out of range.
    fn to_lunar(&self, date: NaiveDate, time_zone: f64) -> Result<LunarDate, CalendarError>;

    /// Converts a lunar `(day, month, year, is_leap_month)` date in `time_zone` to a Gregorian `NaiveDate`.
    ///
    /// Strictly validates `month in 1..=12`, `day in 1..=lunar_month_days`, and leap month membership.
    ///
    /// # Errors
    /// Returns `CalendarError` if any date component, leap flag, or timezone is invalid.
    fn to_solar(
        &self,
        day: i32,
        month: i32,
        year: i32,
        is_leap_month: bool,
        time_zone: f64,
    ) -> Result<NaiveDate, CalendarError>;

    /// Returns the exact day count (`29` or `30`) of the specified lunar month.
    ///
    /// # Errors
    /// Returns `CalendarError` if `month`, `is_leap_month`, or `time_zone` is invalid.
    fn lunar_month_days(
        &self,
        month: i32,
        year: i32,
        is_leap_month: bool,
        time_zone: f64,
    ) -> Result<u8, CalendarError>;

    /// Returns whether lunar `year` contains an astronomical leap month (*năm nhuận*).
    ///
    /// # Errors
    /// Returns `CalendarError::InvalidTimezone` if `time_zone` is invalid.
    fn is_lunar_leap_year(&self, year: i32, time_zone: f64) -> Result<bool, CalendarError>;
}
