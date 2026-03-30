use chrono::{Datelike, NaiveDate};
use vietcalendar_rs::services::{is_solar_leap, to_lunar, to_solar, is_vietnam_holiday};

#[test]
fn test_to_lunar() {
    let date = NaiveDate::from_ymd_opt(2015, 9, 12).unwrap();
    let lunar = to_lunar(date, 7.0);
    assert_eq!(lunar, NaiveDate::from_ymd_opt(2015, 7, 30).unwrap());

    let date = NaiveDate::from_ymd_opt(2001, 1, 1).unwrap();
    let lunar = to_lunar(date, 7.0);
    assert_eq!(lunar, NaiveDate::from_ymd_opt(2000, 12, 7).unwrap());
}

#[test]
fn test_to_lunar2() {
    let date = NaiveDate::from_ymd_opt(2015, 12, 12).unwrap();
    assert_eq!(to_lunar(date, 7.0), NaiveDate::from_ymd_opt(2015, 11, 2).unwrap());

    let date = NaiveDate::from_ymd_opt(2015, 12, 13).unwrap();
    assert_eq!(to_lunar(date, 7.0), NaiveDate::from_ymd_opt(2015, 11, 3).unwrap());

    let date = NaiveDate::from_ymd_opt(2015, 12, 14).unwrap();
    assert_eq!(to_lunar(date, 7.0), NaiveDate::from_ymd_opt(2015, 11, 4).unwrap());
}

#[test]
fn test_to_solar() {
    let date = NaiveDate::from_ymd_opt(2004, 2, 11).unwrap();
    assert_eq!(to_solar(date, 7.0), NaiveDate::from_ymd_opt(2004, 3, 31).unwrap());
}

#[test]
fn test_leap() {
    assert!(!is_solar_leap(2005));
    assert!(is_solar_leap(2004));
}

#[test]
#[ignore]
fn test_holiday() {
    // 30 thang 4 nam 2015
    assert!(is_vietnam_holiday(NaiveDate::from_ymd_opt(2015, 4, 30).unwrap(), 7.0));
    // 30 thang 4 nam 2011
    assert!(is_vietnam_holiday(NaiveDate::from_ymd_opt(2011, 4, 30).unwrap(), 7.0));
    // 02 thang 5 nam 2011
    assert!(is_vietnam_holiday(NaiveDate::from_ymd_opt(2011, 5, 2).unwrap(), 7.0));
    // 03 thang 5 nam 2011
    assert!(is_vietnam_holiday(NaiveDate::from_ymd_opt(2011, 5, 3).unwrap(), 7.0));
    // 04 thang 5 nam 2011
    assert!(!is_vietnam_holiday(NaiveDate::from_ymd_opt(2011, 5, 4).unwrap(), 7.0));
    // 04 thang 5 nam 2015
    assert!(!is_vietnam_holiday(NaiveDate::from_ymd_opt(2015, 5, 4).unwrap(), 7.0));
    // 03 thang 5 nam 2015
    assert!(is_vietnam_holiday(NaiveDate::from_ymd_opt(2015, 5, 3).unwrap(), 7.0));
}
