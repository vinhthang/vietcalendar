use chrono::{Datelike, NaiveDate};
use vietcalendar_rs::services::{is_solar_leap, to_lunar, to_solar, is_vietnam_holiday};
use vietcalendar_rs::calendar::{convert_lunar_to_solar, convert_solar_to_lunar};

#[test]
fn debug_holiday() {
    let time_zone = 7.0;
    println!("2011-05-02 is {}", is_vietnam_holiday(NaiveDate::from_ymd_opt(2011, 5, 2).unwrap(), time_zone));

    let lunar_holidays = [(3, 10), (1, 30), (1, 1), (1, 2), (1, 2)];
    for &(lm, ld) in &lunar_holidays {
        let (d, m, y) = convert_lunar_to_solar(ld, lm, 2011, 0, time_zone);
        println!("Lunar {}/{} 2011 is Solar {}/{}/{}", ld, lm, d, m, y);
    }
}
