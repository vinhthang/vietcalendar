use std::f64::consts::PI;

/**
 * @author duc
 * @param dd
 * @param mm
 * @param yy
 * @return the number of days since 1 January 4713 BC (Julian calendar)
 */
pub fn jd_from_date(dd: i32, mm: i32, yy: i32) -> i32 {
    let a = (14 - mm) / 12;
    let y = yy + 4800 - a;
    let m = mm + 12 * a - 3;
    let mut jd = dd + (153 * m + 2) / 5 + 365 * y + y / 4 - y / 100 + y / 400 - 32045;
    if jd < 2299161 {
        jd = dd + (153 * m + 2) / 5 + 365 * y + y / 4 - 32083;
    }
    //jd = jd - 1721425;
    jd
}

/**
 * http://www.tondering.dk/claus/calendar.html
 * Section: Is there a formula for calculating the Julian day number?
 * @param jd - the number of days since 1 January 4713 BC (Julian calendar)
 * @return
 */
pub fn jd_to_date(jd: i32) -> (i32, i32, i32) {
    let a;
    let b;
    let c;
    if jd > 2299160 {
        // After 5/10/1582, Gregorian calendar
        a = jd + 32044;
        b = (4 * a + 3) / 146097;
        c = a - (b * 146097) / 4;
    } else {
        b = 0;
        c = jd + 32082;
    }
    let d = (4 * c + 3) / 1461;
    let e = c - (1461 * d) / 4;
    let m = (5 * e + 2) / 153;
    let day = e - (153 * m + 2) / 5 + 1;
    let month = m + 3 - 12 * (m / 10);
    let year = b * 100 + d - 4800 + m / 10;
    (day, month, year)
}

/**
 * Solar longitude in degrees
 * Algorithm from: Astronomical Algorithms, by Jean Meeus, 1998
 * @param jdn - number of days since noon UTC on 1 January 4713 BC
 * @return
 */
pub fn sun_longitude(jdn: f64) -> f64 {
    //return CC2K.sunLongitude(jdn);
    sun_longitude_aa98(jdn)
}

pub fn sun_longitude_aa98(jdn: f64) -> f64 {
    let t = (jdn - 2451545.0) / 36525.0; // Time in Julian centuries from 2000-01-01 12:00:00 GMT
    let t2 = t * t;
    let dr = PI / 180.0; // degree to radian
    let m = 357.52910 + 35999.05030 * t - 0.0001559 * t2 - 0.00000048 * t * t2; // mean anomaly, degree
    let l0 = 280.46645 + 36000.76983 * t + 0.0003032 * t2; // mean longitude, degree
    let mut dl = (1.914600 - 0.004817 * t - 0.000014 * t2) * (dr * m).sin();
    dl += (0.019993 - 0.000101 * t) * (dr * 2.0 * m).sin() + 0.000290 * (dr * 3.0 * m).sin();
    let mut l = l0 + dl; // true longitude, degree
    l -= 360.0 * int(l / 360.0) as f64; // Normalize to (0, 360)
    l
}

pub fn new_moon(k: i32) -> f64 {
    //return CC2K.newMoonTime(k);
    new_moon_aa98(k)
}

/**
 * Julian day number of the kth new moon after (or before) the New Moon of 1900-01-01 13:51 GMT.
 * Accuracy: 2 minutes
 * Algorithm from: Astronomical Algorithms, by Jean Meeus, 1998
 * @param k
 * @return the Julian date number (number of days since noon UTC on 1 January 4713 BC) of the New Moon
 */
pub fn new_moon_aa98(k: i32) -> f64 {
    let t = (k as f64) / 1236.85; // Time in Julian centuries from 1900 January 0.5
    let t2 = t * t;
    let t3 = t2 * t;
    let dr = PI / 180.0;
    let mut jd1 = 2415020.75933 + 29.53058868 * (k as f64) + 0.0001178 * t2 - 0.000000155 * t3;
    jd1 += 0.00033 * ((166.56 + 132.87 * t - 0.009173 * t2) * dr).sin(); // Mean new moon
    let m = 359.2242 + 29.10535608 * (k as f64) - 0.0000333 * t2 - 0.00000347 * t3; // Sun's mean anomaly
    let mpr = 306.0253 + 385.81691806 * (k as f64) + 0.0107306 * t2 + 0.00001236 * t3; // Moon's mean anomaly
    let f = 21.2964 + 390.67050646 * (k as f64) - 0.0016528 * t2 - 0.00000239 * t3; // Moon's argument of latitude
    let mut c1 = (0.1734 - 0.000393 * t) * (m * dr).sin() + 0.0021 * (2.0 * dr * m).sin();
    c1 -= 0.4068 * (mpr * dr).sin() - 0.0161 * (dr * 2.0 * mpr).sin();
    c1 -= 0.0004 * (dr * 3.0 * mpr).sin();
    c1 += 0.0104 * (dr * 2.0 * f).sin() - 0.0051 * (dr * (m + mpr)).sin();
    c1 -= 0.0074 * (dr * (m - mpr)).sin() - 0.0004 * (dr * (2.0 * f + m)).sin();
    c1 -= 0.0004 * (dr * (2.0 * f - m)).sin() + 0.0006 * (dr * (2.0 * f + mpr)).sin();
    c1 += 0.0010 * (dr * (2.0 * f - mpr)).sin() + 0.0005 * (dr * (2.0 * mpr + m)).sin();
    let deltat = if t < -11.0 {
        0.001 + 0.000839 * t + 0.0002261 * t2 - 0.00000845 * t3 - 0.000000081 * t * t3
    } else {
        -0.000278 + 0.000265 * t + 0.000262 * t2
    };
    jd1 + c1 - deltat
}

pub fn int(d: f64) -> i32 {
    d.floor() as i32
}

pub fn get_sun_longitude(day_number: i32, time_zone: f64) -> f64 {
    sun_longitude(day_number as f64 - 0.5 - time_zone / 24.0)
}

pub fn get_new_moon_day(k: i32, time_zone: f64) -> i32 {
    let jd = new_moon(k);
    int(jd + 0.5 + time_zone / 24.0)
}

pub fn get_lunar_month_11(yy: i32, time_zone: f64) -> i32 {
    let off = jd_from_date(31, 12, yy) as f64 - 2415021.076998695;
    let k = int(off / 29.530588853);
    let mut nm = get_new_moon_day(k, time_zone);
    let sun_long = int(get_sun_longitude(nm, time_zone) / 30.0);
    if sun_long >= 9 {
        nm = get_new_moon_day(k - 1, time_zone);
    }
    nm
}

pub fn get_leap_month_offset(a11: i32, time_zone: f64) -> i32 {
    let k = int(0.5 + (a11 as f64 - 2415021.076998695) / 29.530588853);
    let mut last; // Month 11 contains point of sun longutide 3*PI/2 (December solstice)
    let mut i = 1; // We start with the month following lunar month 11
    let mut arc = int(get_sun_longitude(get_new_moon_day(k + i, time_zone), time_zone) / 30.0);
    loop {
        last = arc;
        i += 1;
        arc = int(get_sun_longitude(get_new_moon_day(k + i, time_zone), time_zone) / 30.0);
        if arc == last || i >= 14 {
            break;
        }
    }
    i - 1
}

/**
 *
 * @param dd
 * @param mm
 * @param yy
 * @param timeZone
 * @return array of [lunarDay, lunarMonth, lunarYear, leapOrNot]
 */
pub fn convert_solar_to_lunar(dd: i32, mm: i32, yy: i32, time_zone: f64) -> (i32, i32, i32, i32) {
    let day_number = jd_from_date(dd, mm, yy);
    let k = int((day_number as f64 - 2415021.076998695) / 29.530588853);
    let mut month_start = get_new_moon_day(k + 1, time_zone);
    if month_start > day_number {
        month_start = get_new_moon_day(k, time_zone);
    }
    let mut a11 = get_lunar_month_11(yy, time_zone);
    let mut b11 = a11;
    let mut lunar_year;
    if a11 >= month_start {
        lunar_year = yy;
        a11 = get_lunar_month_11(yy - 1, time_zone);
    } else {
        lunar_year = yy + 1;
        b11 = get_lunar_month_11(yy + 1, time_zone);
    }
    let lunar_day = day_number - month_start + 1;
    let diff = (month_start - a11) / 29;
    let mut lunar_leap = 0;
    let mut lunar_month = diff + 11;
    if b11 - a11 > 365 {
        let leap_month_diff = get_leap_month_offset(a11, time_zone);
        if diff >= leap_month_diff {
            lunar_month = diff + 10;
            if diff == leap_month_diff {
                lunar_leap = 1;
            }
        }
    }
    if lunar_month > 12 {
        lunar_month -= 12;
    }
    if lunar_month >= 11 && diff < 4 {
        lunar_year -= 1;
    }
    (lunar_day, lunar_month, lunar_year, lunar_leap)
}

pub fn convert_lunar_to_solar(
    lunar_day: i32,
    lunar_month: i32,
    lunar_year: i32,
    lunar_leap: i32,
    time_zone: f64,
) -> (i32, i32, i32) {
    let a11;
    let b11;
    if lunar_month < 11 {
        a11 = get_lunar_month_11(lunar_year - 1, time_zone);
        b11 = get_lunar_month_11(lunar_year, time_zone);
    } else {
        a11 = get_lunar_month_11(lunar_year, time_zone);
        b11 = get_lunar_month_11(lunar_year + 1, time_zone);
    }
    let k = int(0.5 + (a11 as f64 - 2415021.076998695) / 29.530588853);
    let mut off = lunar_month - 11;
    if off < 0 {
        off += 12;
    }
    if b11 - a11 > 365 {
        let leap_off = get_leap_month_offset(a11, time_zone);
        let mut leap_month = leap_off - 2;
        if leap_month < 0 {
            leap_month += 12;
        }
        if lunar_leap != 0 && lunar_month != leap_month {
            // Invalid input
            return (0, 0, 0);
        } else if lunar_leap != 0 || off >= leap_off {
            off += 1;
        }
    } else if lunar_leap != 0 {
        // Non-leap year has no leap months
        return (0, 0, 0);
    }
    let month_start = get_new_moon_day(k + off, time_zone);
    jd_to_date(month_start + lunar_day - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jd_roundtrip() {
        let test_dates = [
            (1, 1, 2000),
            (29, 2, 2004),
            (15, 8, 1945),
            (30, 4, 1975),
            (17, 8, 2026),
            (15, 10, 1582), // Gregorian transition
            (4, 10, 1582),  // Julian calendar
        ];
        for &(d, m, y) in &test_dates {
            let jd = jd_from_date(d, m, y);
            let (rd, rm, ry) = jd_to_date(jd);
            assert_eq!(
                (rd, rm, ry),
                (d, m, y),
                "Failed roundtrip for date: {d}/{m}/{y}"
            );
        }
    }

    #[test]
    fn test_sun_longitude_range() {
        for jd in [2451545.0, 2451545.0 + 100.0, 2451545.0 + 365.25 * 10.0] {
            let l = sun_longitude(jd);
            assert!(
                l >= 0.0 && l < 360.0,
                "Longitude {l} out of bounds for JD {jd}"
            );
        }
    }

    #[test]
    fn test_new_moon_calculation() {
        // k = 0 corresponds to 1900-01-01
        let nm0 = new_moon(0);
        assert!(nm0 > 2415000.0 && nm0 < 2415050.0);

        let nm1 = new_moon(1);
        let diff = nm1 - nm0;
        // Synodic month is approx 29.53 days
        assert!((diff - 29.53).abs() < 1.0);
    }

    #[test]
    fn test_solar_to_lunar_and_back() {
        // 2024-02-10 is Lunar New Year (1/1/2024 Giap Thin)
        let (ld, lm, ly, leap) = convert_solar_to_lunar(10, 2, 2024, 7.0);
        assert_eq!((ld, lm, ly, leap), (1, 1, 2024, 0));

        let (sd, sm, sy) = convert_lunar_to_solar(1, 1, 2024, 0, 7.0);
        assert_eq!((sd, sm, sy), (10, 2, 2024));
    }
}
