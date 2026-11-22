//! `VietnamCalendarPlugin`: non-blocking `moka`-cached astronomical calendar provider.

pub mod layout;
pub mod math;

use std::cell::RefCell;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use chrono::{Datelike, NaiveDate};
use moka::sync::Cache;
use serde::Deserialize;
use serde_json::Value;

use crate::contracts::calendar::CalendarService;
use crate::contracts::models::{CalendarError, LunarDate};
use crate::kernel::context::Context;
use crate::kernel::effect::BoxFuture;
use crate::kernel::error::KernelError;
use crate::kernel::fiber::{Plugin, PluginMeta};
use crate::plugins::calendar_vn::layout::LunarSpan;
use crate::plugins::calendar_vn::math::{jd_from_date, jd_to_date};

#[derive(Debug, Clone, Deserialize)]
struct CalendarConfig {
    #[serde(default = "default_cache_capacity")]
    cache_capacity: u64,
}

fn default_cache_capacity() -> u64 {
    4096
}

impl Default for CalendarConfig {
    fn default() -> Self {
        Self {
            cache_capacity: default_cache_capacity(),
        }
    }
}

const MIN_SUPPORTED_YEAR: i32 = -260_000;
const MAX_SUPPORTED_YEAR: i32 = 260_000;

/// Validates `year` stays within safe Gregorian/Julian conversion bounds.
#[inline]
fn validate_year(year: i32) -> Result<(), CalendarError> {
    if !(MIN_SUPPORTED_YEAR..=MAX_SUPPORTED_YEAR).contains(&year) {
        return Err(CalendarError::ConversionOutOfRange);
    }
    Ok(())
}

/// Validates `time_zone` and quantizes it to integer quarter-hours (`0.25h`) for cache keying.
fn quantize_timezone(time_zone: f64) -> Result<(i16, f64), CalendarError> {
    if !time_zone.is_finite() || !(-12.0..=14.0).contains(&time_zone) {
        return Err(CalendarError::InvalidTimezone(time_zone));
    }
    let quarters = (time_zone * 4.0).round() as i16;
    Ok((quarters, f64::from(quarters) / 4.0))
}

static NEXT_ENGINE_ID: AtomicU64 = AtomicU64::new(1);

const L1_CAPACITY: usize = 4;

struct SpanL1Cache {
    entries: [Option<(u64, i32, i16, LunarSpan)>; L1_CAPACITY],
    cursor: usize,
}

impl SpanL1Cache {
    const fn new() -> Self {
        Self {
            entries: [None; L1_CAPACITY],
            cursor: 0,
        }
    }

    #[inline]
    fn get(&self, engine_id: u64, anchor_year: i32, tz_q: i16) -> Option<LunarSpan> {
        for slot in &self.entries {
            if let Some((eid, y, tz, span)) = *slot {
                if eid == engine_id && y == anchor_year && tz == tz_q {
                    return Some(span);
                }
            }
        }
        None
    }

    #[inline]
    fn insert(&mut self, engine_id: u64, anchor_year: i32, tz_q: i16, span: LunarSpan) {
        let idx = self.cursor % L1_CAPACITY;
        if let Some(target) = self.entries.get_mut(idx) {
            *target = Some((engine_id, anchor_year, tz_q, span));
        }
        self.cursor = self.cursor.wrapping_add(1);
    }
}

thread_local! {
    static SPAN_L1: RefCell<SpanL1Cache> = const { RefCell::new(SpanL1Cache::new()) };
}

/// Concrete `CalendarService` implementation backed by a bounded `moka::sync::Cache` and per-thread L1 cache.
pub struct VietnamCalendarEngine {
    engine_id: u64,
    spans: Cache<(i32, i16), Arc<LunarSpan>>,
}

impl VietnamCalendarEngine {
    /// Creates a new calendar engine with `cache_capacity` maximum cached `LunarSpan` entries.
    pub fn new(cache_capacity: u64) -> Self {
        Self {
            engine_id: NEXT_ENGINE_ID.fetch_add(1, Ordering::Relaxed),
            spans: Cache::new(cache_capacity.max(32)),
        }
    }

    #[inline]
    fn get_span(&self, anchor_year: i32, tz_quarters: i16, tz_hours: f64) -> LunarSpan {
        if let Ok(Some(hit)) =
            SPAN_L1.try_with(|c| c.borrow().get(self.engine_id, anchor_year, tz_quarters))
        {
            return hit;
        }
        let shared = self.spans.get_with((anchor_year, tz_quarters), || {
            Arc::new(LunarSpan::compute(anchor_year, tz_hours))
        });
        let span = *shared;
        let _ = SPAN_L1.try_with(|c| {
            if let Ok(mut guard) = c.try_borrow_mut() {
                guard.insert(self.engine_id, anchor_year, tz_quarters, span);
            }
        });
        span
    }
}

impl CalendarService for VietnamCalendarEngine {
    fn to_lunar(&self, date: NaiveDate, time_zone: f64) -> Result<LunarDate, CalendarError> {
        validate_year(date.year())?;
        let (tz_q, tz_h) = quantize_timezone(time_zone)?;
        let jd = jd_from_date(date.day() as i32, date.month() as i32, date.year());

        // Fast-locate anchor year: Month 11 of `y` starts late Nov / Dec in Gregorian year `y`,
        // so dates in Jan..=mid-Nov almost always reside inside `anchor_year = y - 1`.
        let primary_anchor = if date.month() < 11 || (date.month() == 11 && date.day() < 20) {
            date.year() - 1
        } else {
            date.year()
        };
        let mut span = self.get_span(primary_anchor, tz_q, tz_h);
        if jd < span.start_jd {
            span = self.get_span(primary_anchor - 1, tz_q, tz_h);
        } else if jd >= span.end_jd {
            span = self.get_span(primary_anchor + 1, tz_q, tz_h);
        }

        if let Some((day, slot)) = span.find_by_jd(jd) {
            return Ok(LunarDate {
                day,
                month: slot.month,
                year: slot.lunar_year,
                is_leap: slot.is_leap,
            });
        }

        // Guard against any edge-case boundary shift across neighboring anchor years.
        for offset in [-1, 1] {
            let neighbor = self.get_span(primary_anchor + offset, tz_q, tz_h);
            if let Some((day, slot)) = neighbor.find_by_jd(jd) {
                return Ok(LunarDate {
                    day,
                    month: slot.month,
                    year: slot.lunar_year,
                    is_leap: slot.is_leap,
                });
            }
        }

        Err(CalendarError::ConversionOutOfRange)
    }

    fn to_solar(
        &self,
        day: i32,
        month: i32,
        year: i32,
        is_leap_month: bool,
        time_zone: f64,
    ) -> Result<NaiveDate, CalendarError> {
        validate_year(year)?;
        let (tz_q, tz_h) = quantize_timezone(time_zone)?;
        if !(1..=12).contains(&month) {
            return Err(CalendarError::InvalidLunarMonth(month));
        }

        let anchor_year = if month < 11 { year - 1 } else { year };
        let span = self.get_span(anchor_year, tz_q, tz_h);
        let Some(slot) = span.find_by_lunar(month, year, is_leap_month) else {
            return Err(CalendarError::InvalidLeapMonth { month, year });
        };

        if day < 1 || day > i32::from(slot.days) {
            return Err(CalendarError::InvalidLunarDay {
                day,
                month,
                year,
                max_days: slot.days,
            });
        }

        let (sd, sm, sy) = jd_to_date(slot.start_jd + day - 1);
        let sm_u32 = u32::try_from(sm).map_err(|_| CalendarError::ConversionOutOfRange)?;
        let sd_u32 = u32::try_from(sd).map_err(|_| CalendarError::ConversionOutOfRange)?;
        NaiveDate::from_ymd_opt(sy, sm_u32, sd_u32).ok_or(CalendarError::ConversionOutOfRange)
    }

    fn lunar_month_days(
        &self,
        month: i32,
        year: i32,
        is_leap_month: bool,
        time_zone: f64,
    ) -> Result<u8, CalendarError> {
        validate_year(year)?;
        let (tz_q, tz_h) = quantize_timezone(time_zone)?;
        if !(1..=12).contains(&month) {
            return Err(CalendarError::InvalidLunarMonth(month));
        }
        let anchor_year = if month < 11 { year - 1 } else { year };
        let span = self.get_span(anchor_year, tz_q, tz_h);
        let Some(slot) = span.find_by_lunar(month, year, is_leap_month) else {
            return Err(CalendarError::InvalidLeapMonth { month, year });
        };
        Ok(slot.days)
    }

    fn is_lunar_leap_year(&self, year: i32, time_zone: f64) -> Result<bool, CalendarError> {
        validate_year(year)?;
        let (tz_q, tz_h) = quantize_timezone(time_zone)?;
        let prev_span = self.get_span(year - 1, tz_q, tz_h);
        if prev_span.has_leap_for_year(year) {
            return Ok(true);
        }
        let curr_span = self.get_span(year, tz_q, tz_h);
        Ok(curr_span.has_leap_for_year(year))
    }
}

/// Cordis plugin providing `dyn CalendarService`.
#[derive(Debug, Default)]
pub struct VietnamCalendarPlugin;

impl Plugin for VietnamCalendarPlugin {
    fn meta(&self) -> PluginMeta {
        PluginMeta::new("calendar_vn").provides::<dyn CalendarService>()
    }

    fn mount<'a>(&'a self, ctx: Context, config: Value) -> BoxFuture<'a, Result<(), KernelError>> {
        Box::pin(async move {
            let cfg: CalendarConfig = serde_json::from_value(config)
                .map_err(|e| KernelError::ConfigError(format!("calendar_vn config: {e}")))?;
            let engine: Arc<dyn CalendarService> =
                Arc::new(VietnamCalendarEngine::new(cfg.cache_capacity));
            ctx.provide::<dyn CalendarService>(engine);
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::calendar_vn::math::convert_solar_to_lunar_raw;
    use chrono::Duration;

    #[test]
    fn test_exhaustive_15000_day_aa98_equivalence_and_roundtrip() {
        let engine = VietnamCalendarEngine::new(2048);
        let start = NaiveDate::from_ymd_opt(1995, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2035, 12, 31).unwrap();

        for tz in [7.0, 8.0, -5.0] {
            let mut cur = start;
            while cur <= end {
                let actual = engine.to_lunar(cur, tz).unwrap();
                let (rd, rm, ry, rleap) = convert_solar_to_lunar_raw(
                    cur.day() as i32,
                    cur.month() as i32,
                    cur.year(),
                    tz,
                );
                assert_eq!(
                    (actual.day, actual.month, actual.year, actual.is_leap),
                    (rd, rm, ry, rleap != 0),
                    "mismatch on solar {cur} tz={tz}"
                );

                let back = engine
                    .to_solar(actual.day, actual.month, actual.year, actual.is_leap, tz)
                    .unwrap();
                assert_eq!(back, cur, "roundtrip failed for {actual:?} tz={tz}");

                cur += Duration::days(1);
            }
        }
    }

    #[test]
    fn test_strict_29_day_month_and_validation_boundaries() {
        let engine = VietnamCalendarEngine::new(128);

        // Leap month 2 in 2004
        assert_eq!(
            engine.to_solar(11, 2, 2004, true, 7.0).unwrap(),
            NaiveDate::from_ymd_opt(2004, 3, 31).unwrap()
        );
        // Invalid leap month
        assert!(matches!(
            engine.to_solar(1, 1, 2024, true, 7.0).unwrap_err(),
            CalendarError::InvalidLeapMonth {
                month: 1,
                year: 2024
            }
        ));

        // Find a 29-day month (e.g., check 2024 lunar months) and verify day 30 is strictly rejected!
        let mut found_29 = false;
        for m in 1..=12 {
            let days = engine.lunar_month_days(m, 2024, false, 7.0).unwrap();
            if days == 29 {
                found_29 = true;
                let err = engine.to_solar(30, m, 2024, false, 7.0).unwrap_err();
                assert_eq!(
                    err,
                    CalendarError::InvalidLunarDay {
                        day: 30,
                        month: m,
                        year: 2024,
                        max_days: 29,
                    }
                );
            }
        }
        assert!(found_29);

        // Out-of-range month & invalid timezone
        assert_eq!(
            engine.to_solar(1, 13, 2024, false, 7.0).unwrap_err(),
            CalendarError::InvalidLunarMonth(13)
        );
        assert!(matches!(
            engine
                .to_lunar(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(), f64::NAN)
                .unwrap_err(),
            CalendarError::InvalidTimezone(_)
        ));
        assert!(matches!(
            engine
                .to_lunar(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(), 15.0)
                .unwrap_err(),
            CalendarError::InvalidTimezone(_)
        ));
    }

    #[test]
    fn test_is_lunar_leap_year_accuracy() {
        let engine = VietnamCalendarEngine::new(128);
        assert!(engine.is_lunar_leap_year(2004, 7.0).unwrap());
        assert!(engine.is_lunar_leap_year(2020, 7.0).unwrap());
        assert!(engine.is_lunar_leap_year(2023, 7.0).unwrap());
        assert!(!engine.is_lunar_leap_year(2024, 7.0).unwrap());
        assert!(engine.is_lunar_leap_year(2025, 7.0).unwrap());
        assert!(engine.is_lunar_leap_year(2033, 7.0).unwrap());
    }

    #[test]
    fn test_extreme_year_bounds_return_out_of_range_without_panic() {
        let engine = VietnamCalendarEngine::new(128);
        assert_eq!(
            engine.to_solar(1, 11, i32::MAX, false, 7.0),
            Err(CalendarError::ConversionOutOfRange)
        );
        assert_eq!(
            engine.to_solar(1, 1, i32::MIN, false, 7.0),
            Err(CalendarError::ConversionOutOfRange)
        );
        assert_eq!(
            engine.to_solar(1, 1, 10_000_000, false, 7.0),
            Err(CalendarError::ConversionOutOfRange)
        );
        assert_eq!(
            engine.lunar_month_days(1, 10_000_000, false, 7.0),
            Err(CalendarError::ConversionOutOfRange)
        );
        assert_eq!(
            engine.is_lunar_leap_year(i32::MIN, 7.0),
            Err(CalendarError::ConversionOutOfRange)
        );
        assert_eq!(
            engine.is_lunar_leap_year(i32::MAX, 7.0),
            Err(CalendarError::ConversionOutOfRange)
        );
    }

    #[test]
    fn test_nov_dec_boundary_and_concurrent_threads_match_aa98() {
        let engine = Arc::new(VietnamCalendarEngine::new(2048));
        for y in 2020..=2030 {
            let mut cur = NaiveDate::from_ymd_opt(y, 11, 1).unwrap();
            let end = NaiveDate::from_ymd_opt(y, 12, 31).unwrap();
            while cur <= end {
                let actual = engine.to_lunar(cur, 7.0).unwrap();
                let (rd, rm, ry, rleap) = convert_solar_to_lunar_raw(
                    cur.day() as i32,
                    cur.month() as i32,
                    cur.year(),
                    7.0,
                );
                assert_eq!(
                    (actual.day, actual.month, actual.year, actual.is_leap),
                    (rd, rm, ry, rleap != 0),
                    "boundary mismatch on {cur}"
                );
                let back = engine
                    .to_solar(actual.day, actual.month, actual.year, actual.is_leap, 7.0)
                    .unwrap();
                assert_eq!(back, cur);
                cur += Duration::days(1);
            }
        }

        std::thread::scope(|s| {
            for tid in 0..4 {
                let eng = Arc::clone(&engine);
                s.spawn(move || {
                    for day_offset in 0..100 {
                        let d = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()
                            + Duration::days((tid * 31 + day_offset * 3) as i64);
                        let lunar = eng.to_lunar(d, 7.0).unwrap();
                        let solar = eng
                            .to_solar(lunar.day, lunar.month, lunar.year, lunar.is_leap, 7.0)
                            .unwrap();
                        assert_eq!(solar, d);
                    }
                });
            }
        });
    }
}
