//! 24 Solar Terms (*Tiết Khí*) astronomical solver backed by bounded `moka` cache.

use std::sync::Arc;

use chrono::{Datelike, NaiveDate, NaiveDateTime};
use moka::sync::Cache;

use crate::contracts::canchi::{ActiveSolarTermResult, SolarTermInfo};
use crate::contracts::models::CalendarError;
use crate::plugins::calendar_vn::math::{jd_from_date, jd_to_date, sun_longitude_aa98};

const MIN_SUPPORTED_YEAR: i32 = -260_000;
const MAX_SUPPORTED_YEAR: i32 = 260_000;
const SOLAR_TERMS_CACHE_CAPACITY: u64 = 256;

struct StaticTermSpec {
    id: &'static str,
    name_vi: &'static str,
    name_en: &'static str,
    longitude_deg: u16,
    is_major_tiet: bool,
}

const SOLAR_TERMS: &[StaticTermSpec] = &[
    StaticTermSpec {
        id: "tieu_han",
        name_vi: "Tiểu Hàn",
        name_en: "Minor Cold",
        longitude_deg: 285,
        is_major_tiet: true,
    },
    StaticTermSpec {
        id: "dai_han",
        name_vi: "Đại Hàn",
        name_en: "Major Cold",
        longitude_deg: 300,
        is_major_tiet: false,
    },
    StaticTermSpec {
        id: "lap_xuan",
        name_vi: "Lập Xuân",
        name_en: "Start of Spring",
        longitude_deg: 315,
        is_major_tiet: true,
    },
    StaticTermSpec {
        id: "vu_thuy",
        name_vi: "Vũ Thủy",
        name_en: "Rain Water",
        longitude_deg: 330,
        is_major_tiet: false,
    },
    StaticTermSpec {
        id: "kinh_trap",
        name_vi: "Kinh Trập",
        name_en: "Awakening of Insects",
        longitude_deg: 345,
        is_major_tiet: true,
    },
    StaticTermSpec {
        id: "xuan_phan",
        name_vi: "Xuân Phân",
        name_en: "Vernal Equinox",
        longitude_deg: 0,
        is_major_tiet: false,
    },
    StaticTermSpec {
        id: "thanh_minh",
        name_vi: "Thanh Minh",
        name_en: "Pure Brightness",
        longitude_deg: 15,
        is_major_tiet: true,
    },
    StaticTermSpec {
        id: "coc_vu",
        name_vi: "Cốc Vũ",
        name_en: "Grain Rain",
        longitude_deg: 30,
        is_major_tiet: false,
    },
    StaticTermSpec {
        id: "lap_ha",
        name_vi: "Lập Hạ",
        name_en: "Start of Summer",
        longitude_deg: 45,
        is_major_tiet: true,
    },
    StaticTermSpec {
        id: "tieu_man",
        name_vi: "Tiểu Mãn",
        name_en: "Grain Full",
        longitude_deg: 60,
        is_major_tiet: false,
    },
    StaticTermSpec {
        id: "mang_chung",
        name_vi: "Mang Chủng",
        name_en: "Grain in Ear",
        longitude_deg: 75,
        is_major_tiet: true,
    },
    StaticTermSpec {
        id: "ha_chi",
        name_vi: "Hạ Chí",
        name_en: "Summer Solstice",
        longitude_deg: 90,
        is_major_tiet: false,
    },
    StaticTermSpec {
        id: "tieu_thu",
        name_vi: "Tiểu Thử",
        name_en: "Minor Heat",
        longitude_deg: 105,
        is_major_tiet: true,
    },
    StaticTermSpec {
        id: "dai_thu",
        name_vi: "Đại Thử",
        name_en: "Major Heat",
        longitude_deg: 120,
        is_major_tiet: false,
    },
    StaticTermSpec {
        id: "lap_thu",
        name_vi: "Lập Thu",
        name_en: "Start of Autumn",
        longitude_deg: 135,
        is_major_tiet: true,
    },
    StaticTermSpec {
        id: "xu_thu",
        name_vi: "Xử Thử",
        name_en: "End of Heat",
        longitude_deg: 150,
        is_major_tiet: false,
    },
    StaticTermSpec {
        id: "bach_lo",
        name_vi: "Bạch Lộ",
        name_en: "White Dew",
        longitude_deg: 165,
        is_major_tiet: true,
    },
    StaticTermSpec {
        id: "thu_phan",
        name_vi: "Thu Phân",
        name_en: "Autumnal Equinox",
        longitude_deg: 180,
        is_major_tiet: false,
    },
    StaticTermSpec {
        id: "han_lo",
        name_vi: "Hàn Lộ",
        name_en: "Cold Dew",
        longitude_deg: 195,
        is_major_tiet: true,
    },
    StaticTermSpec {
        id: "suong_giang",
        name_vi: "Sương Giáng",
        name_en: "Frost's Descent",
        longitude_deg: 210,
        is_major_tiet: false,
    },
    StaticTermSpec {
        id: "lap_dong",
        name_vi: "Lập Đông",
        name_en: "Start of Winter",
        longitude_deg: 225,
        is_major_tiet: true,
    },
    StaticTermSpec {
        id: "tieu_tuyet",
        name_vi: "Tiểu Tuyết",
        name_en: "Minor Snow",
        longitude_deg: 240,
        is_major_tiet: false,
    },
    StaticTermSpec {
        id: "dai_tuyet",
        name_vi: "Đại Tuyết",
        name_en: "Major Snow",
        longitude_deg: 255,
        is_major_tiet: true,
    },
    StaticTermSpec {
        id: "dong_chi",
        name_vi: "Đông Chí",
        name_en: "Winter Solstice",
        longitude_deg: 270,
        is_major_tiet: false,
    },
];

/// Validates year stays within calendar safety bounds.
pub(crate) fn validate_year_bounds(year: i32) -> Result<(), CalendarError> {
    if !(MIN_SUPPORTED_YEAR..=MAX_SUPPORTED_YEAR).contains(&year) {
        return Err(CalendarError::ConversionOutOfRange);
    }
    Ok(())
}

/// Validates and quantizes timezone to quarter hours (`-12.0..=14.0`).
pub(crate) fn validate_timezone(time_zone: f64) -> Result<(i16, f64), CalendarError> {
    if !time_zone.is_finite() || !(-12.0..=14.0).contains(&time_zone) {
        return Err(CalendarError::InvalidTimezone(time_zone));
    }
    let quarters = (time_zone * 4.0).round() as i16;
    Ok((quarters, f64::from(quarters) / 4.0))
}

/// Validates clock `hour` and `minute` bounds.
pub(crate) fn validate_hour_minute(hour: u8, minute: u8) -> Result<(), CalendarError> {
    if hour >= 24 {
        return Err(CalendarError::InvalidSolarDate(format!(
            "hour {hour} out of 0..=23 range"
        )));
    }
    if minute >= 60 {
        return Err(CalendarError::InvalidSolarDate(format!(
            "minute {minute} out of 0..=59 range"
        )));
    }
    Ok(())
}

/// Converts local `(date, hour, minute)` with `tz_hours` offset into UTC Unix timestamp seconds.
pub(crate) fn local_to_unix_seconds(
    date: NaiveDate,
    hour: u8,
    minute: u8,
    tz_hours: f64,
) -> Result<i64, CalendarError> {
    validate_year_bounds(date.year())?;
    validate_hour_minute(hour, minute)?;
    let tz_secs = (tz_hours * 3600.0).round() as i64;
    let time = chrono::NaiveTime::from_hms_opt(u32::from(hour), u32::from(minute), 0)
        .ok_or_else(|| CalendarError::InvalidSolarDate("invalid clock time".to_string()))?;
    let dt = NaiveDateTime::new(date, time);
    Ok(dt.and_utc().timestamp() - tz_secs)
}

#[inline]
fn signed_angular_diff(lon: f64, target: f64) -> f64 {
    (lon - target + 540.0).rem_euclid(360.0) - 180.0
}

fn solve_solar_term_ut(guess_jdn_ut: f64, target_deg: f64) -> f64 {
    let mut lo = guess_jdn_ut - 10.0;
    let mut hi = guess_jdn_ut + 10.0;
    for _ in 0..3 {
        if signed_angular_diff(sun_longitude_aa98(lo), target_deg) <= 0.0
            && signed_angular_diff(sun_longitude_aa98(hi), target_deg) >= 0.0
        {
            break;
        }
        lo -= 6.0;
        hi += 6.0;
    }
    for _ in 0..45 {
        let mid = 0.5 * (lo + hi);
        if signed_angular_diff(sun_longitude_aa98(mid), target_deg) < 0.0 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    0.5 * (lo + hi)
}

fn format_offset(tz_hours: f64) -> String {
    let total_mins = (tz_hours * 60.0).round() as i32;
    let sign = if total_mins >= 0 { '+' } else { '-' };
    let abs_mins = total_mins.abs();
    format!("{sign}{:02}:{:02}", abs_mins / 60, abs_mins % 60)
}

fn jdn_ut_to_local_parts(jdn_ut: f64, tz_hours: f64) -> (String, String, i64) {
    let unix_seconds = ((jdn_ut - 2440587.5) * 86_400.0).round() as i64;
    let jdn_local = jdn_ut + 0.5 + tz_hours / 24.0;
    let mut day_num = jdn_local.floor() as i32;
    let mut secs_in_day = ((jdn_local - f64::from(day_num)) * 86_400.0).round() as i32;
    if secs_in_day >= 86_400 {
        secs_in_day -= 86_400;
        day_num += 1;
    }
    let (dd, mm, yy) = jd_to_date(day_num);
    let h = secs_in_day / 3600;
    let m = (secs_in_day % 3600) / 60;
    let s = secs_in_day % 60;
    let date_str = format!("{yy:04}-{mm:02}-{dd:02}");
    let tz_str = format_offset(tz_hours);
    let ts_str = format!("{date_str}T{h:02}:{m:02}:{s:02}{tz_str}");
    (date_str, ts_str, unix_seconds)
}

/// Bounded `moka`-cached 24 Solar Terms calculator.
pub(crate) struct SolarTermsEngine {
    cache: Cache<(i32, i16), Arc<Vec<SolarTermInfo>>>,
}

impl Default for SolarTermsEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl SolarTermsEngine {
    /// Creates a new bounded solar terms cache engine.
    pub(crate) fn new() -> Self {
        Self {
            cache: Cache::new(SOLAR_TERMS_CACHE_CAPACITY),
        }
    }

    /// Resolves and caches all 24 solar terms occurring within Gregorian `year` at `time_zone`.
    pub(crate) fn year_terms(
        &self,
        year: i32,
        time_zone: f64,
    ) -> Result<Arc<Vec<SolarTermInfo>>, CalendarError> {
        validate_year_bounds(year)?;
        let (tz_q, tz_hours) = validate_timezone(time_zone)?;

        Ok(self.cache.get_with((year, tz_q), || {
            let jan1_ut = f64::from(jd_from_date(1, 1, year)) - 0.5 - tz_hours / 24.0;
            let lon_jan1 = sun_longitude_aa98(jan1_ut);
            let mut results = Vec::with_capacity(SOLAR_TERMS.len());

            for (idx, spec) in SOLAR_TERMS.iter().enumerate() {
                let target = f64::from(spec.longitude_deg);
                let delta_deg = (target - lon_jan1).rem_euclid(360.0);
                let guess_ut = jan1_ut + delta_deg * (365.2422 / 360.0);
                let exact_ut = solve_solar_term_ut(guess_ut, target);
                let (date, timestamp, unix_seconds) = jdn_ut_to_local_parts(exact_ut, tz_hours);
                results.push(SolarTermInfo {
                    index: idx as u8,
                    id: spec.id.to_string(),
                    name_vi: spec.name_vi.to_string(),
                    name_en: spec.name_en.to_string(),
                    solar_longitude_deg: spec.longitude_deg,
                    date,
                    timestamp,
                    unix_seconds,
                    is_major_tiet: spec.is_major_tiet,
                });
            }
            results.sort_by_key(|item| item.unix_seconds);
            for (idx, item) in results.iter_mut().enumerate() {
                item.index = idx as u8;
            }
            Arc::new(results)
        }))
    }

    /// Resolves active and next solar term for local date and clock time.
    pub(crate) fn active_term(
        &self,
        date: NaiveDate,
        hour: u8,
        minute: u8,
        time_zone: f64,
    ) -> Result<ActiveSolarTermResult, CalendarError> {
        let (_, tz_hours) = validate_timezone(time_zone)?;
        let query_ts = local_to_unix_seconds(date, hour, minute, tz_hours)?;
        let jd_mid = jd_from_date(date.day() as i32, date.month() as i32, date.year());
        let frac = (f64::from(hour) + f64::from(minute) / 60.0) / 24.0;
        let jdn_ut = f64::from(jd_mid) - 0.5 - tz_hours / 24.0 + frac;
        let sun_longitude_deg = sun_longitude_aa98(jdn_ut);

        let terms = self.year_terms(date.year(), tz_hours)?;
        let first = terms
            .first()
            .ok_or(CalendarError::ConversionOutOfRange)?
            .clone();
        let last = terms
            .last()
            .ok_or(CalendarError::ConversionOutOfRange)?
            .clone();

        let (current_term, next_term) = if query_ts < first.unix_seconds {
            let prev_terms = self.year_terms(date.year() - 1, tz_hours)?;
            let prev_last = prev_terms
                .last()
                .ok_or(CalendarError::ConversionOutOfRange)?
                .clone();
            (prev_last, first)
        } else if query_ts >= last.unix_seconds {
            let next_terms = self.year_terms(date.year() + 1, tz_hours)?;
            let next_first = next_terms
                .first()
                .ok_or(CalendarError::ConversionOutOfRange)?
                .clone();
            (last, next_first)
        } else {
            let mut pos = 0usize;
            for (i, t) in terms.iter().enumerate() {
                if t.unix_seconds <= query_ts {
                    pos = i;
                } else {
                    break;
                }
            }
            let cur = terms
                .get(pos)
                .ok_or(CalendarError::ConversionOutOfRange)?
                .clone();
            let nxt = terms
                .get(pos + 1)
                .ok_or(CalendarError::ConversionOutOfRange)?
                .clone();
            (cur, nxt)
        };

        Ok(ActiveSolarTermResult {
            date: date.format("%Y-%m-%d").to_string(),
            hour,
            minute,
            time_zone: tz_hours,
            sun_longitude_deg,
            current_term,
            next_term,
        })
    }
}
