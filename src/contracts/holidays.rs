//! Service contract (`HolidayService`) for data-driven per-year Vietnam holiday lookups.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::NaiveDate;

use crate::contracts::models::{CalendarError, YearHolidaySchedule};

/// Disposer handle returned when a plugin registers a reversible year holiday override.
pub type HolidayDisposer = Box<dyn FnOnce() + Send>;

/// Data-driven Vietnam holiday capability provided by `VietnamHolidaysPlugin`.
pub trait HolidayService: Send + Sync + 'static {
    /// Returns `true` if `date` is a configured holiday or non-working weekend.
    ///
    /// Unconfigured years fall back strictly to weekends (`Saturday`/`Sunday`).
    fn is_holiday(&self, date: NaiveDate) -> bool;

    /// Validates `time_zone` (`-12.0..=14.0`, finite) and evaluates whether `date` is a holiday.
    ///
    /// # Errors
    /// Returns `CalendarError::InvalidTimezone` for invalid `time_zone`.
    fn is_holiday_with_timezone(
        &self,
        date: NaiveDate,
        time_zone: f64,
    ) -> Result<bool, CalendarError>;

    /// Returns the active immutable schedule for `year` (or a weekends-only fallback schedule).
    fn year_holidays(&self, year: i32) -> Arc<YearHolidaySchedule>;

    /// Returns `true` if an explicit schedule (YAML or plugin override) exists for `year`.
    fn has_schedule(&self, year: i32) -> bool;

    /// Atomically replaces the base loaded YAML schedules and invalidates stale cache entries.
    fn replace_schedules(&self, schedules: HashMap<i32, Arc<YearHolidaySchedule>>);

    /// Registers an overlay schedule for `schedule.year`, returning a reversible LIFO disposer.
    fn register_year_schedule(&self, schedule: YearHolidaySchedule) -> HolidayDisposer;
}
