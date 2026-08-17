use chrono::NaiveDate;
use vietcalendar_rs::models::LunarDate;
use vietcalendar_rs::services::{is_solar_leap, is_vietnam_holiday, to_lunar, to_solar};

#[test]
fn test_to_lunar() {
    let date = NaiveDate::from_ymd_opt(2015, 9, 12).unwrap();
    let lunar = to_lunar(date, 7.0);
    assert_eq!(
        lunar,
        LunarDate {
            day: 30,
            month: 7,
            year: 2015,
            is_leap: false,
        }
    );

    let date = NaiveDate::from_ymd_opt(2001, 1, 1).unwrap();
    let lunar = to_lunar(date, 7.0);
    assert_eq!(
        lunar,
        LunarDate {
            day: 7,
            month: 12,
            year: 2000,
            is_leap: false,
        }
    );
}

#[test]
fn test_to_lunar2() {
    let date = NaiveDate::from_ymd_opt(2015, 12, 12).unwrap();
    assert_eq!(
        to_lunar(date, 7.0),
        LunarDate {
            day: 2,
            month: 11,
            year: 2015,
            is_leap: false,
        }
    );

    let date = NaiveDate::from_ymd_opt(2015, 12, 13).unwrap();
    assert_eq!(
        to_lunar(date, 7.0),
        LunarDate {
            day: 3,
            month: 11,
            year: 2015,
            is_leap: false,
        }
    );

    let date = NaiveDate::from_ymd_opt(2015, 12, 14).unwrap();
    assert_eq!(
        to_lunar(date, 7.0),
        LunarDate {
            day: 4,
            month: 11,
            year: 2015,
            is_leap: false,
        }
    );
}

#[test]
fn test_to_solar() {
    // 11/2/2004 (leap month 2 in 2004)
    assert_eq!(
        to_solar(11, 2, 2004, true, 7.0),
        NaiveDate::from_ymd_opt(2004, 3, 31)
    );
    // Regular month in 2004 (non-leap)
    assert_eq!(
        to_solar(11, 2, 2004, false, 7.0),
        NaiveDate::from_ymd_opt(2004, 3, 1)
    );
}

#[test]
fn test_leap() {
    assert!(!is_solar_leap(2005));
    assert!(is_solar_leap(2004));
}

#[test]
fn test_roundtrip_conversion() {
    let original = NaiveDate::from_ymd_opt(2023, 8, 15).unwrap();
    let lunar = to_lunar(original, 7.0);
    let converted_back = to_solar(lunar.day, lunar.month, lunar.year, lunar.is_leap, 7.0).unwrap();
    assert_eq!(original, converted_back);
}

#[test]
fn test_holiday() {
    // 30 thang 4 nam 2015 (Thursday)
    assert!(is_vietnam_holiday(NaiveDate::from_ymd_opt(2015, 4, 30).unwrap(), 7.0));
    // 30 thang 4 nam 2011 (Saturday)
    assert!(is_vietnam_holiday(NaiveDate::from_ymd_opt(2011, 4, 30).unwrap(), 7.0));
    // 02 thang 5 nam 2011 (Monday - compensatory for 30/4)
    assert!(is_vietnam_holiday(NaiveDate::from_ymd_opt(2011, 5, 2).unwrap(), 7.0));
    // 03 thang 5 nam 2011 (Tuesday - compensatory for 1/5)
    assert!(is_vietnam_holiday(NaiveDate::from_ymd_opt(2011, 5, 3).unwrap(), 7.0));
    // 04 thang 5 nam 2011 (Wednesday - regular working day)
    assert!(!is_vietnam_holiday(NaiveDate::from_ymd_opt(2011, 5, 4).unwrap(), 7.0));
    // 04 thang 5 nam 2015 (Monday - regular working day)
    assert!(!is_vietnam_holiday(NaiveDate::from_ymd_opt(2015, 5, 4).unwrap(), 7.0));
    // 03 thang 5 nam 2015 (Sunday - weekend)
    assert!(is_vietnam_holiday(NaiveDate::from_ymd_opt(2015, 5, 3).unwrap(), 7.0));
}

