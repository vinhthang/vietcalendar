# Design: Vietnam Public Holidays and Compensatory Leave Engine

Sources: [spec.md](./spec.md), [decisions.md](./decisions.md)

## Components

| Requirement / AC | Path and symbol | Responsibility / change |
| --- | --- | --- |
| REQ-005 / AC-005-1, AC-005-2 | `src/services.rs::is_solar_holiday`, `is_vietnam_holiday` | Weekend (`Sat`/`Sun`) and fixed solar holiday checks (`01/01`, `30/04`, `01/05`, `02/09`). |
| REQ-006 / AC-006-1, AC-006-2 | `src/services.rs::get_all_holidays_for_year` | Queries `to_solar` across lunar years `solar_year - 1..=solar_year + 1` for Giỗ Tổ Hùng Vương (`10/03` Lunar) and Tết (Eve + `01/01`–`03/01` Lunar). |
| REQ-007 / AC-007-1, AC-007-2 | `src/services.rs::get_all_holidays_for_year`, `is_vietnam_holiday`, `HOLIDAY_CACHE` | Weekend compensatory rollover (*nghỉ bù*) and `OnceLock<RwLock<HashMap<i32, HashSet<NaiveDate>>>>` year cache. |

## Interfaces and flow

- **Public entrypoints (`src/services.rs`):** `pub fn is_vietnam_holiday(date: NaiveDate, time_zone: f64) -> bool` and `pub fn is_solar_holiday(date: NaiveDate) -> bool`.
- **Internal generator (`src/services.rs`):** `fn get_all_holidays_for_year(solar_year: i32, time_zone: f64) -> HashSet<NaiveDate>`:
  1. Collects fixed solar dates (`1/1`, `4/30`, `5/1`, `9/2`).
  2. Iterates `ly` in `[solar_year - 1, solar_year, solar_year + 1]`, invoking `to_solar` for `10/3` Lunar and `1/1`–`3/1` Lunar (deriving Eve as `tet1 - Duration::days(1)`) and retaining dates with `.year() == solar_year`.
  3. Sorts and deduplicates `public_holidays`, builds `all_holidays: HashSet<NaiveDate>`, then for each weekend holiday advances `comp = holiday + 1d` until `!Sat && !Sun && !all_holidays.contains(&comp)`.
- **Cache lookup:** `is_vietnam_holiday` returns `true` immediately for `Sat`/`Sun`, acquires `HOLIDAY_CACHE` read lock for `date.year()`, or computes and inserts under write lock on miss.

## Boundaries and recovery

- **Solar year boundary:** Checking `ly in [solar_year - 1, solar_year, solar_year + 1]` catches Tết spanning January/February across Gregorian vs. Lunar year transitions (e.g., when Lunar `1/1` falls in January/February of `solar_year` or Eve falls on `12/31`).
- **Compensatory chain:** Sequential weekend holidays (e.g. `2011-04-30` Sat and `2011-05-01` Sun) avoid colliding because each inserted compensatory day becomes part of `all_holidays`.
- **Timezone cache limitation (`TD-02`):** `HOLIDAY_CACHE` keys only on `year: i32`, so the first `time_zone` queried for a given year persists for subsequent calls in that process.

## Dependencies and unresolved design work

None.
