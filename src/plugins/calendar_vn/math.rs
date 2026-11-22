//! Pure Jean Meeus AA98 and Claus Tøndering Julian Day astronomical math.

use std::f64::consts::PI;

/// Converts a Gregorian or Julian calendar date `(dd, mm, yy)` to an integer Julian Day Number.
///
/// Applies the October 1582 Gregorian calendar reform cutoff (`jd < 2299161`).
pub fn jd_from_date(dd: i32, mm: i32, yy: i32) -> i32 {
    let a = (14 - mm) / 12;
    let y = yy + 4800 - a;
    let m = mm + 12 * a - 3;
    let mut jd = dd + (153 * m + 2) / 5 + 365 * y + y / 4 - y / 100 + y / 400 - 32045;
    if jd < 2299161 {
        jd = dd + (153 * m + 2) / 5 + 365 * y + y / 4 - 32083;
    }
    jd
}

/// Converts an integer Julian Day Number back into `(day, month, year)`.
///
/// Handles both Julian (`jd <= 2299160`) and Gregorian (`jd > 2299160`) eras.
pub fn jd_to_date(jd: i32) -> (i32, i32, i32) {
    let b;
    let c;
    if jd > 2299160 {
        let a = jd + 32044;
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

/// Computes the floor integer of `d` matching astronomical truncation semantics.
#[inline]
pub(crate) fn floor_i32(d: f64) -> i32 {
    d.floor() as i32
}

/// Computes the apparent solar longitude in `[0.0, 360.0)` degrees via Jean Meeus AA98.
pub fn sun_longitude_aa98(jdn: f64) -> f64 {
    let t = (jdn - 2451545.0) / 36525.0;
    let t2 = t * t;
    let dr = PI / 180.0;
    let m = 357.52910 + 35999.05030 * t - 0.0001559 * t2 - 0.00000048 * t * t2;
    let l0 = 280.46645 + 36000.76983 * t + 0.0003032 * t2;
    let mut dl = (1.914600 - 0.004817 * t - 0.000014 * t2) * (dr * m).sin();
    dl += (0.019993 - 0.000101 * t) * (dr * 2.0 * m).sin() + 0.000290 * (dr * 3.0 * m).sin();
    let mut l = l0 + dl;
    l -= 360.0 * floor_i32(l / 360.0) as f64;
    l
}

/// Computes the Julian Day Number of the `k`-th new moon relative to 1900-01-01 13:51 GMT
/// using Jean Meeus *Astronomical Algorithms* (1998).
pub fn new_moon_aa98(k: i32) -> f64 {
    let t = (k as f64) / 1236.85;
    let t2 = t * t;
    let t3 = t2 * t;
    let dr = PI / 180.0;
    let mut jd1 = 2415020.75933 + 29.53058868 * (k as f64) + 0.0001178 * t2 - 0.000000155 * t3;
    jd1 += 0.00033 * ((166.56 + 132.87 * t - 0.009173 * t2) * dr).sin();
    let m = 359.2242 + 29.10535608 * (k as f64) - 0.0000333 * t2 - 0.00000347 * t3;
    let mpr = 306.0253 + 385.81691806 * (k as f64) + 0.0107306 * t2 + 0.00001236 * t3;
    let f = 21.2964 + 390.67050646 * (k as f64) - 0.0016528 * t2 - 0.00000239 * t3;
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

/// Evaluates the sun longitude at midnight local time (`time_zone` hours) for `day_number`.
pub fn get_sun_longitude(day_number: i32, time_zone: f64) -> f64 {
    sun_longitude_aa98(day_number as f64 - 0.5 - time_zone / 24.0)
}

/// Returns the local Julian Day Number on which the `k`-th new moon falls.
pub fn get_new_moon_day(k: i32, time_zone: f64) -> i32 {
    let jd = new_moon_aa98(k);
    floor_i32(jd + 0.5 + time_zone / 24.0)
}

/// Finds the start Julian Day Number of Lunar Month 11 (containing the December solstice)
/// for solar year `yy` in `time_zone`.
pub fn get_lunar_month_11(yy: i32, time_zone: f64) -> i32 {
    let off = jd_from_date(31, 12, yy) as f64 - 2415021.076998695;
    let k = floor_i32(off / 29.530588853);
    let mut nm = get_new_moon_day(k, time_zone);
    let sun_long = floor_i32(get_sun_longitude(nm, time_zone) / 30.0);
    if sun_long >= 9 {
        nm = get_new_moon_day(k - 1, time_zone);
    }
    nm
}

/// Computes the 1-based offset from Lunar Month 11 (`a11`) to the leap month lacking a major solar term.
pub fn get_leap_month_offset(a11: i32, time_zone: f64) -> i32 {
    let k = floor_i32(0.5 + (a11 as f64 - 2415021.076998695) / 29.530588853);
    let mut last;
    let mut i = 1;
    let mut arc =
        floor_i32(get_sun_longitude(get_new_moon_day(k + i, time_zone), time_zone) / 30.0);
    loop {
        last = arc;
        i += 1;
        arc = floor_i32(get_sun_longitude(get_new_moon_day(k + i, time_zone), time_zone) / 30.0);
        if arc == last || i >= 14 {
            break;
        }
    }
    i - 1
}

/// Reference raw solar-to-lunar conversion used to verify `LunarSpan` equivalence.
#[cfg(test)]
pub(crate) fn convert_solar_to_lunar_raw(
    dd: i32,
    mm: i32,
    yy: i32,
    time_zone: f64,
) -> (i32, i32, i32, i32) {
    let day_number = jd_from_date(dd, mm, yy);
    let mut k = floor_i32((day_number as f64 - 2415021.076998695) / 29.530588853) + 1;
    let mut month_start = get_new_moon_day(k, time_zone);
    while month_start > day_number {
        k -= 1;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jd_roundtrip_and_1582_boundary() {
        let test_dates = [
            (1, 1, 2000),
            (29, 2, 2004),
            (15, 8, 1945),
            (30, 4, 1975),
            (17, 8, 2026),
            (15, 10, 1582),
            (4, 10, 1582),
        ];
        for &(d, m, y) in &test_dates {
            let jd = jd_from_date(d, m, y);
            assert_eq!(jd_to_date(jd), (d, m, y), "failed roundtrip {d}/{m}/{y}");
        }
        assert!(jd_from_date(4, 10, 1582) < 2299161);
        assert!(jd_from_date(15, 10, 1582) >= 2299161);
    }

    #[test]
    fn test_sun_longitude_and_new_moon_bounds() {
        for jd in [2451545.0, 2451545.0 + 100.0, 2451545.0 + 365.25 * 10.0] {
            let l = sun_longitude_aa98(jd);
            assert!((0.0..360.0).contains(&l), "longitude {l} out of range");
        }
        let nm0 = new_moon_aa98(0);
        assert!(nm0 > 2415000.0 && nm0 < 2415050.0);
        let diff = new_moon_aa98(1) - nm0;
        assert!((diff - 29.53).abs() < 1.0);
    }
}
