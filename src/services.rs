use chrono::{Datelike, NaiveDate};

use crate::calendar::{convert_lunar_to_solar, convert_solar_to_lunar};

pub fn is_solar_leap(year: i32) -> bool {
    // 0, 3, 6, 9, 11, 14, 17
    let modulo = year % 19;
    matches!(modulo, 3 | 6 | 9 | 11 | 14 | 17)
}

pub fn to_lunar(date: NaiveDate, time_zone: f64) -> NaiveDate {
    let (d, m, y, _) = convert_solar_to_lunar(
        date.day() as i32,
        date.month() as i32,
        date.year(),
        time_zone,
    );
    NaiveDate::from_ymd_opt(y, m as u32, d as u32).expect("Invalid lunar date")
}

pub fn to_solar(date: NaiveDate, time_zone: f64) -> NaiveDate {
    let leap = if is_solar_leap(date.year()) { 1 } else { 0 };
    let (d, m, y) = convert_lunar_to_solar(
        date.day() as i32,
        date.month() as i32,
        date.year(),
        leap,
        time_zone,
    );
    NaiveDate::from_ymd_opt(y, m as u32, d as u32).expect("Invalid solar date")
}

pub fn is_vietnam_holiday(date: NaiveDate, time_zone: f64) -> bool {
    is_solar_holiday(date) || is_in_lunar_holiday(date, time_zone)
}

pub fn is_solar_holiday(date: NaiveDate) -> bool {
    use chrono::Weekday;
    let weekday = date.weekday();
    if weekday == Weekday::Sat || weekday == Weekday::Sun {
        return true;
    }
    let d = date.day();
    let m = date.month();
    // 9/2, 4/30, 5/1, 1/1
    (m == 9 && d == 2) || (m == 4 && d == 30) || (m == 5 && d == 1) || (m == 1 && d == 1)
}

pub fn is_in_lunar_holiday(date: NaiveDate, time_zone: f64) -> bool {
    let lunar_holidays = [(3, 10), (1, 30), (1, 1), (1, 2), (1, 3)];
    // Actually the java implementation converts these lunar months/days for the current polar year to solar and checks equality.
    for &(lm, ld) in &lunar_holidays {
        // We assume lunar_leap = 0 for holidays
        let (d, m, y) = convert_lunar_to_solar(ld, lm, date.year(), 0, time_zone);
        if let Some(solar_date) = NaiveDate::from_ymd_opt(y, m as u32, d as u32) {
            if solar_date == date {
                return true;
            }
        }
    }
    false
}
