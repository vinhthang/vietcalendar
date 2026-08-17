use chrono::{Datelike, Duration, NaiveDate, Weekday};
use std::collections::HashSet;

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

pub fn to_solar(
    day: i32,
    month: i32,
    year: i32,
    is_leap_month: bool,
    time_zone: f64,
) -> Option<NaiveDate> {
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
                comp += Duration::days(1);
            }
            all_holidays.insert(comp);
        }
    }

    all_holidays
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metonic_leap_years() {
        // Years in 19-year Metonic cycle with leap month: 0, 3, 6, 9, 11, 14, 17
        assert!(is_solar_leap(2020)); // 2020 % 19 = 6
        assert!(is_solar_leap(2023)); // 2023 % 19 = 9
        assert!(is_solar_leap(2025)); // 2025 % 19 = 11
        assert!(!is_solar_leap(2021)); // 2021 % 19 = 7
        assert!(!is_solar_leap(2022)); // 2022 % 19 = 8
        assert!(!is_solar_leap(2024)); // 2024 % 19 = 10
    }

    #[test]
    fn test_to_lunar_and_to_solar() {
        let solar_date = NaiveDate::from_ymd_opt(2024, 2, 10).unwrap();
        let lunar = to_lunar(solar_date, 7.0);
        assert_eq!(lunar.day, 1);
        assert_eq!(lunar.month, 1);
        assert_eq!(lunar.year, 2024);
        assert!(!lunar.is_leap);

        let converted = to_solar(lunar.day, lunar.month, lunar.year, lunar.is_leap, 7.0).unwrap();
        assert_eq!(converted, solar_date);
    }

    #[test]
    fn test_to_solar_invalid_leap_month() {
        // In 2024, month 1 is not a leap month. Asking for leap month 1 should return None
        let invalid = to_solar(1, 1, 2024, true, 7.0);
        assert_eq!(invalid, None);
    }

    #[test]
    fn test_solar_holiday_check() {
        // New Year's Day (1/1)
        assert!(is_solar_holiday(
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()
        ));
        // Reunification Day (30/4)
        assert!(is_solar_holiday(
            NaiveDate::from_ymd_opt(2024, 4, 30).unwrap()
        ));
        // International Workers' Day (1/5)
        assert!(is_solar_holiday(
            NaiveDate::from_ymd_opt(2024, 5, 1).unwrap()
        ));
        // National Day (2/9)
        assert!(is_solar_holiday(
            NaiveDate::from_ymd_opt(2024, 9, 2).unwrap()
        ));
        // Regular working day (e.g. Wednesday 15/5/2024)
        assert!(!is_solar_holiday(
            NaiveDate::from_ymd_opt(2024, 5, 15).unwrap()
        ));
    }

    #[test]
    fn test_vietnam_holidays_with_compensatory() {
        let tz = 7.0;
        // In 2011: 30/4 (Sat) & 1/5 (Sun) -> 2/5 (Mon) & 3/5 (Tue) are compensatory holidays
        assert!(is_vietnam_holiday(
            NaiveDate::from_ymd_opt(2011, 4, 30).unwrap(),
            tz
        ));
        assert!(is_vietnam_holiday(
            NaiveDate::from_ymd_opt(2011, 5, 1).unwrap(),
            tz
        ));
        assert!(is_vietnam_holiday(
            NaiveDate::from_ymd_opt(2011, 5, 2).unwrap(),
            tz
        ));
        assert!(is_vietnam_holiday(
            NaiveDate::from_ymd_opt(2011, 5, 3).unwrap(),
            tz
        ));
        assert!(!is_vietnam_holiday(
            NaiveDate::from_ymd_opt(2011, 5, 4).unwrap(),
            tz
        ));

        // Tet Giap Thin 2024: 1/1 Lunar was 2024-02-10 (Saturday)
        // Eve was 2024-02-09 (Friday) -> holiday
        assert!(is_vietnam_holiday(
            NaiveDate::from_ymd_opt(2024, 2, 9).unwrap(),
            tz
        ));
        // Tet day 1 (2024-02-10, Sat) -> holiday
        assert!(is_vietnam_holiday(
            NaiveDate::from_ymd_opt(2024, 2, 10).unwrap(),
            tz
        ));
    }
}
