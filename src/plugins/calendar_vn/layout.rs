//! Immutable per-year lunar layout spans (`LunarSpan`) for O(1) solar/lunar conversion.

use crate::plugins::calendar_vn::math::{
    floor_i32, get_leap_month_offset, get_lunar_month_11, get_new_moon_day,
};

/// Metadata for one synodic lunar month within a `LunarSpan`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LunarMonthSlot {
    /// Julian Day Number of the first day (mùng 1) of this lunar month.
    pub start_jd: i32,
    /// Actual synodic length (`29` or `30` days).
    pub days: u8,
    /// Lunar month number (`1..=12`).
    pub month: i32,
    /// Lunar year owning this month (`anchor_year` for months 11–12 before Tet, `anchor_year + 1` after).
    pub lunar_year: i32,
    /// Whether this slot is the leap month (*tháng nhuận*).
    pub is_leap: bool,
}

const EMPTY_SLOT: LunarMonthSlot = LunarMonthSlot {
    start_jd: 0,
    days: 0,
    month: 0,
    lunar_year: 0,
    is_leap: false,
};

/// Immutable lunar layout spanning `[a11(anchor_year), a11(anchor_year + 1))` for a timezone.
///
/// Precomputing these 12 or 13 month slots once per `(anchor_year, tz_quarter_hours)` makes
/// solar-to-lunar, lunar-to-solar, month-length (`29`/`30`), and leap-year lookups O(1) without
/// repeating Jean Meeus trigonometry on every request.
#[derive(Debug, Clone, Copy)]
pub struct LunarSpan {
    /// Starting Julian Day Number (`a11` of `anchor_year`).
    pub start_jd: i32,
    /// Exclusive ending Julian Day Number (`a11` of `anchor_year + 1`).
    pub end_jd: i32,
    slots: [LunarMonthSlot; 13],
    len: usize,
}

impl LunarSpan {
    /// Returns the active 12 (normal) or 13 (leap) lunar month slots covering `[start_jd, end_jd)`.
    #[inline]
    pub fn slots(&self) -> &[LunarMonthSlot] {
        self.slots.get(..self.len).unwrap_or(&[])
    }

    /// Computes the exact lunar month table between Month 11 of `anchor_year` and Month 11 of `anchor_year + 1`.
    pub fn compute(anchor_year: i32, time_zone: f64) -> Self {
        let a11 = get_lunar_month_11(anchor_year, time_zone);
        let b11 = get_lunar_month_11(anchor_year + 1, time_zone);
        let k0 = floor_i32(0.5 + (a11 as f64 - 2415021.076998695) / 29.530588853);

        let has_leap = b11 - a11 > 365;
        let leap_diff = if has_leap {
            Some(get_leap_month_offset(a11, time_zone))
        } else {
            None
        };
        let num_months: i32 = if has_leap { 13 } else { 12 };

        let mut slots = [EMPTY_SLOT; 13];
        let mut len: usize = 0;
        let mut prev_start = a11;

        for idx in 0..num_months {
            let next_start = if idx + 1 == num_months {
                b11
            } else {
                get_new_moon_day(k0 + idx + 1, time_zone)
            };
            let days = (next_start - prev_start).clamp(28, 31) as u8;

            let (raw_month, is_leap) = match leap_diff {
                Some(ld) if idx >= ld => (idx + 10, idx == ld),
                _ => (idx + 11, false),
            };
            let month = if raw_month > 12 {
                raw_month - 12
            } else {
                raw_month
            };
            let mut lunar_year = anchor_year + 1;
            if month >= 11 && idx < 4 {
                lunar_year -= 1;
            }

            if let Some(target) = slots.get_mut(len) {
                *target = LunarMonthSlot {
                    start_jd: prev_start,
                    days,
                    month,
                    lunar_year,
                    is_leap,
                };
                len += 1;
            }
            prev_start = next_start;
        }

        Self {
            start_jd: a11,
            end_jd: b11,
            slots,
            len,
        }
    }

    /// Finds the slot containing Julian Day Number `jd` (`start_jd <= jd < end_jd`).
    #[inline]
    pub fn find_by_jd(&self, jd: i32) -> Option<(i32, &LunarMonthSlot)> {
        let active = self.slots();
        let idx = active.partition_point(|s| s.start_jd <= jd);
        if idx == 0 {
            return None;
        }
        let slot = active.get(idx - 1)?;
        if jd < slot.start_jd + i32::from(slot.days) {
            Some((jd - slot.start_jd + 1, slot))
        } else {
            None
        }
    }

    /// Finds the exact slot matching `(month, lunar_year, is_leap)`.
    #[inline]
    pub fn find_by_lunar(
        &self,
        month: i32,
        lunar_year: i32,
        is_leap: bool,
    ) -> Option<&LunarMonthSlot> {
        self.slots()
            .iter()
            .find(|s| s.month == month && s.lunar_year == lunar_year && s.is_leap == is_leap)
    }

    /// Returns whether any slot owned by `lunar_year` inside this span is a leap month.
    #[inline]
    pub fn has_leap_for_year(&self, lunar_year: i32) -> bool {
        self.slots()
            .iter()
            .any(|s| s.lunar_year == lunar_year && s.is_leap)
    }
}
