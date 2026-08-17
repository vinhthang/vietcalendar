use std::collections::HashSet;
use chrono::{Datelike, Duration, NaiveDate, Weekday};

use crate::calendar::{convert_lunar_to_solar, convert_solar_to_lunar};
use crate::models::LunarDate;

pub fn is_solar_leap(year: i32) -> bool {
    let modulo = year % 19;
    matches!(modulo, 0 | 3 | 6 | 9 | 11 | 14 | 17)
}

pub fn to_lunar(date: NaiveDate, time_zone: f64) -> LunarDate {
    let (d, m, y, leap) = convert_solar_to_lunar(
        date.day() as i32,
        date.month() as i32,
        date.year(),
        time_zone,
    );
    LunarDate {
        day: d,
        month: m,
        year: y,
        is_leap: leap != 0,
    }
}

pub fn to_solar(day: i32, month: i32, year: i32, is_leap_month: bool, time_zone: f64) -> Option<NaiveDate> {
    let leap = if is_leap_month { 1 } else { 0 };
    let (d, m, y) = convert_lunar_to_solar(day, month, year, leap, time_zone);
    if d == 0 && m == 0 && y == 0 {
        None
    } else {
        NaiveDate::from_ymd_opt(y, m as u32, d as u32)
    }
}

pub fn is_vietnam_holiday(date: NaiveDate, time_zone: f64) -> bool {
    let weekday = date.weekday();
    if weekday == Weekday::Sat || weekday == Weekday::Sun {
        return true;
    }

    let year = date.year();
    let holidays = get_all_holidays_for_year(year, time_zone);
    holidays.contains(&date)
}

pub fn is_solar_holiday(date: NaiveDate) -> bool {
    let weekday = date.weekday();
    if weekday == Weekday::Sat || weekday == Weekday::Sun {
        return true;
    }
    let d = date.day();
    let m = date.month();
    // 1/1, 30/4, 1/5, 2/9
    (m == 1 && d == 1) || (m == 4 && d == 30) || (m == 5 && d == 1) || (m == 9 && d == 2)
}

/// Computes all public holidays and compensatory days for a given solar year
fn get_all_holidays_for_year(solar_year: i32, time_zone: f64) -> HashSet<NaiveDate> {
    let mut public_holidays: Vec<NaiveDate> = Vec::new();

    // 1. Fixed solar public holidays
    for &(m, d) in &[(1, 1), (4, 30), (5, 1), (9, 2)] {
        if let Some(d) = NaiveDate::from_ymd_opt(solar_year, m, d) {
            public_holidays.push(d);
        }
    }

    // 2. Lunar public holidays for solar year (check lunar year Y-1, Y, Y+1)
    for ly in [solar_year - 1, solar_year, solar_year + 1] {
        // Hung Kings: 10/3 Lunar
        if let Some(hk) = to_solar(10, 3, ly, false, time_zone) {
            if hk.year() == solar_year {
                public_holidays.push(hk);
            }
        }

        // Tet Lunar New Year: 1/1 Lunar
        if let Some(tet1) = to_solar(1, 1, ly, false, time_zone) {
            // New Year's Eve is 1 day before 1/1
            let eve = tet1 - Duration::days(1);
            if eve.year() == solar_year {
                public_holidays.push(eve);
            }
            if tet1.year() == solar_year {
                public_holidays.push(tet1);
            }
            // Tet days: 2/1, 3/1 Lunar
            for day in 2..=3 {
                if let Some(tet_day) = to_solar(day, 1, ly, false, time_zone) {
                    if tet_day.year() == solar_year {
                        public_holidays.push(tet_day);
                    }
                }
            }
        }
    }

    public_holidays.sort();
    public_holidays.dedup();

    let mut all_holidays: HashSet<NaiveDate> = public_holidays.iter().copied().collect();

    // 3. Calculate compensatory days for holidays falling on weekends
    for &holiday in &public_holidays {
        let wd = holiday.weekday();
        if wd == Weekday::Sat || wd == Weekday::Sun {
            // Find the next available non-weekend, non-holiday weekday
            let mut comp = holiday + Duration::days(1);
            while comp.weekday() == Weekday::Sat
                || comp.weekday() == Weekday::Sun
                || all_holidays.contains(&comp)
            {
                comp = comp + Duration::days(1);
            }
            all_holidays.insert(comp);
        }
    }

    all_holidays
}

