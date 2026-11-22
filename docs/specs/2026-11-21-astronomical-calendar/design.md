# Design: Vietnamese Luni-Solar Astronomical Calendar Engine

Sources: [spec.md](./spec.md), [decisions.md](./decisions.md)

## Components

| Requirement / AC | Path and symbol | Responsibility / change |
| --- | --- | --- |
| REQ-001 / AC-001-1, AC-001-2 | `src/calendar.rs::jd_from_date`, `jd_to_date` | Integer Julian Day Number bidirectional conversion across Gregorian/Julian 1582 boundary (`2299160`/`2299161`). |
| REQ-002 / AC-002-1, AC-002-2 | `src/calendar.rs::sun_longitude`, `sun_longitude_aa98`, `new_moon`, `new_moon_aa98`, `int`, `get_sun_longitude`, `get_new_moon_day` | Jean Meeus AA98 solar longitude (`[0.0, 360.0)`) and lunation instant math with historical `deltat` threshold (`t < -11.0`). |
| REQ-003 / AC-003-1, AC-003-2 | `src/calendar.rs::get_lunar_month_11`, `get_leap_month_offset`, `convert_solar_to_lunar`, `convert_lunar_to_solar`; `src/services.rs::to_lunar`, `to_solar`; `src/models.rs::LunarDate` | Bidirectional solar/lunar date conversion, leap-month identification, `LunarDate` model, and `Option<NaiveDate>` mapping. |
| REQ-004 / AC-004-1, AC-004-2 | `src/services.rs::is_solar_leap`; `src/models.rs::DateMonthYear`, `LunarDate` | Metonic `year % 19` leap-month cadence predicate and Serde JSON alias/optional-field contract. |

## Interfaces and flow

- **Calendar primitives (`src/calendar.rs`):** `jd_from_date(dd, mm, yy) -> i32`, `jd_to_date(jd) -> (i32, i32, i32)`, `sun_longitude_aa98(jdn: f64) -> f64`, `new_moon_aa98(k: i32) -> f64`, `convert_solar_to_lunar(dd, mm, yy, time_zone) -> (i32, i32, i32, i32)`, `convert_lunar_to_solar(lunar_day, lunar_month, lunar_year, lunar_leap, time_zone) -> (i32, i32, i32)`.
- **Service wrappers (`src/services.rs`):** `to_lunar(date: NaiveDate, time_zone: f64) -> LunarDate` and `to_solar(day, month, year, is_leap_month: bool, time_zone: f64) -> Option<NaiveDate>`.
- **Domain & DTO structs (`src/models.rs`):** `LunarDate { day: i32, month: i32, year: i32, is_leap: bool }` and `DateMonthYear { dd: i32, mm: i32, yyyy: i32, leap: Option<bool> }` (`#[serde(alias = "mm", alias = "MM")]`, `skip_serializing_if = "Option::is_none"` on `leap`).

## Boundaries and recovery

- **Invalid leap-month requests:** `convert_lunar_to_solar` checks `b11 - a11 > 365` and `lunar_month != leap_month`; invalid `lunar_leap != 0` calls return `(0, 0, 0)`, which `services::to_solar` translates to `None`.
- **Unchecked day/month baseline behavior (`TD-03`):** `convert_lunar_to_solar` does not enforce `lunar_month in 1..=12` or cap `lunar_day` to actual 29/30-day synodic length; day `30` on a 29-day month rolls forward via `jd_to_date(month_start + lunar_day - 1)`.
- **Metonic predicate (`TD-04`):** `is_solar_leap(year)` evaluates `year % 19` against `{0, 3, 6, 9, 11, 14, 17}` without overflow or panic.

## Dependencies and unresolved design work

None.
