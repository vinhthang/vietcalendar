//! `CanChiSolarPlugin`: Shared Can Chi & 24 Solar Terms engine backed by `CalendarService`.

pub mod canchi;
pub mod solar_terms;

use std::sync::Arc;

use chrono::{Datelike, NaiveDate};
use serde_json::Value;

use crate::contracts::calendar::CalendarService;
use crate::contracts::canchi::{
    ActiveSolarTermResult, CanChiBreakdown, CanChiPair, CanChiSolarService, SolarTermInfo,
};
use crate::contracts::models::CalendarError;
use crate::kernel::context::Context;
use crate::kernel::effect::BoxFuture;
use crate::kernel::error::KernelError;
use crate::kernel::fiber::{Plugin, PluginMeta};
use crate::plugins::calendar_vn::math::jd_from_date;
use crate::plugins::canchi_solar_vn::solar_terms::{
    local_to_unix_seconds, validate_timezone, validate_year_bounds, SolarTermsEngine,
};

/// Shared Can Chi and 24 Solar Terms implementation.
pub struct CanChiSolarEngine {
    calendar: Arc<dyn CalendarService>,
    terms: SolarTermsEngine,
}

impl CanChiSolarEngine {
    /// Builds a new [`CanChiSolarEngine`] over an existing [`CalendarService`].
    pub fn new(calendar: Arc<dyn CalendarService>) -> Self {
        Self {
            calendar,
            terms: SolarTermsEngine::new(),
        }
    }
}

impl CanChiSolarService for CanChiSolarEngine {
    fn day_can_chi(&self, date: NaiveDate) -> Result<CanChiPair, CalendarError> {
        validate_year_bounds(date.year())?;
        let jd = jd_from_date(date.day() as i32, date.month() as i32, date.year());
        Ok(canchi::pair_from_jd(jd))
    }

    fn full_can_chi(
        &self,
        date: NaiveDate,
        hour: u8,
        minute: u8,
        time_zone: f64,
    ) -> Result<CanChiBreakdown, CalendarError> {
        let (_, tz_hours) = validate_timezone(time_zone)?;
        let query_ts = local_to_unix_seconds(date, hour, minute, tz_hours)?;
        let lunar = self.calendar.to_lunar(date, tz_hours)?;
        let day_cc = self.day_can_chi(date)?;
        let (hour_cc, hour_stem_day_shifted) = canchi::pair_for_hour(day_cc.stem_index, hour)?;

        let lunar_year_cc = canchi::pair_for_year(lunar.year);
        let lunar_month_cc = canchi::pair_for_month(lunar_year_cc.stem_index, lunar.month)?;

        let year_terms = self.terms.year_terms(date.year(), tz_hours)?;
        let lap_xuan = year_terms
            .iter()
            .find(|t| t.id == "lap_xuan")
            .ok_or(CalendarError::ConversionOutOfRange)?;
        let tieu_han = year_terms
            .iter()
            .find(|t| t.id == "tieu_han")
            .ok_or(CalendarError::ConversionOutOfRange)?;

        let (bazi_year, bazi_month) = if query_ts < lap_xuan.unix_seconds {
            let m = if query_ts >= tieu_han.unix_seconds {
                12
            } else {
                11
            };
            (date.year() - 1, m)
        } else {
            let major_count = year_terms
                .iter()
                .filter(|t| {
                    t.is_major_tiet
                        && t.unix_seconds >= lap_xuan.unix_seconds
                        && t.unix_seconds <= query_ts
                })
                .count();
            (date.year(), (major_count as i32).clamp(1, 12))
        };

        let bazi_year_cc = canchi::pair_for_year(bazi_year);
        let bazi_month_cc = canchi::pair_for_month(bazi_year_cc.stem_index, bazi_month)?;
        let active = self.terms.active_term(date, hour, minute, tz_hours)?;

        Ok(CanChiBreakdown {
            solar_date: date.format("%Y-%m-%d").to_string(),
            hour,
            minute,
            time_zone: tz_hours,
            lunar,
            lunar_year_can_chi: lunar_year_cc,
            lunar_month_can_chi: lunar_month_cc,
            bazi_year_can_chi: bazi_year_cc,
            bazi_month_can_chi: bazi_month_cc,
            day_can_chi: day_cc,
            hour_can_chi: hour_cc,
            hour_stem_day_shifted,
            current_solar_term: active.current_term,
            next_solar_term: active.next_term,
        })
    }

    fn solar_terms_for_year(
        &self,
        year: i32,
        time_zone: f64,
    ) -> Result<Vec<SolarTermInfo>, CalendarError> {
        Ok((*self.terms.year_terms(year, time_zone)?).clone())
    }

    fn active_solar_term(
        &self,
        date: NaiveDate,
        hour: u8,
        minute: u8,
        time_zone: f64,
    ) -> Result<ActiveSolarTermResult, CalendarError> {
        self.terms.active_term(date, hour, minute, time_zone)
    }

    fn pair_from_index(&self, index: u8) -> CanChiPair {
        canchi::pair_from_sexagenary_index(usize::from(index))
    }

    fn year_can_chi(&self, year: i32) -> CanChiPair {
        canchi::pair_for_year(year)
    }

    fn month_can_chi(&self, year_stem_index: u8, month: i32) -> Result<CanChiPair, CalendarError> {
        canchi::pair_for_month(year_stem_index, month)
    }
}

/// Cordis plugin providing [`CanChiSolarService`].
pub struct CanChiSolarPlugin;

impl Plugin for CanChiSolarPlugin {
    fn meta(&self) -> PluginMeta {
        PluginMeta::new("canchi_solar_vn")
            .requires::<dyn CalendarService>()
            .provides::<dyn CanChiSolarService>()
    }

    fn mount<'a>(&'a self, ctx: Context, _config: Value) -> BoxFuture<'a, Result<(), KernelError>> {
        Box::pin(async move {
            let calendar = ctx.inject::<dyn CalendarService>()?;
            let service: Arc<dyn CanChiSolarService> = Arc::new(CanChiSolarEngine::new(calendar));
            ctx.provide::<dyn CanChiSolarService>(service);
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::NaiveDate;

    use crate::contracts::calendar::CalendarService;
    use crate::contracts::canchi::CanChiSolarService;
    use crate::plugins::calendar_vn::VietnamCalendarEngine;
    use crate::plugins::canchi_solar_vn::CanChiSolarEngine;

    fn make_engine() -> CanChiSolarEngine {
        let cal: Arc<dyn CalendarService> = Arc::new(VietnamCalendarEngine::new(256));
        CanChiSolarEngine::new(cal)
    }

    #[test]
    fn test_day_can_chi_reference_dates() {
        let engine = make_engine();
        let cases = [
            ((2000, 1, 1), "mau_ngo", "Mậu Ngọ"),
            ((1984, 2, 2), "binh_dan", "Bính Dần"),
            ((2024, 2, 10), "giap_thin", "Giáp Thìn"),
            ((2024, 2, 3), "dinh_dau", "Đinh Dậu"),
        ];
        for ((y, m, d), exp_id, exp_vi) in cases {
            let date = NaiveDate::from_ymd_opt(y, m, d).unwrap();
            let pair = engine.day_can_chi(date).unwrap();
            assert_eq!(pair.id, exp_id, "failed id for {y}-{m:02}-{d:02}");
            assert_eq!(pair.name_vi, exp_vi);
        }

        let giap_ty = engine.pair_from_index(0);
        assert_eq!(giap_ty.id, "giap_ty");
        assert_eq!(giap_ty.nap_am.id, "hai_trung_kim");
        assert_eq!(giap_ty.nap_am.name_vi, "Hải Trung Kim");
        assert_eq!(engine.pair_from_index(54).id, "mau_ngo");
    }

    #[test]
    fn test_full_can_chi_and_lap_xuan_hour_rollover() {
        let engine = make_engine();
        // 2024-02-10 12:00 (Mùng 1 Tết Giáp Thìn): Giáp Thìn / Bính Dần / Giáp Thìn / Canh Ngọ
        let tet = engine
            .full_can_chi(NaiveDate::from_ymd_opt(2024, 2, 10).unwrap(), 12, 0, 7.0)
            .unwrap();
        assert_eq!(tet.lunar_year_can_chi.id, "giap_thin");
        assert_eq!(tet.lunar_month_can_chi.id, "binh_dan");
        assert_eq!(tet.bazi_year_can_chi.id, "giap_thin");
        assert_eq!(tet.bazi_month_can_chi.id, "binh_dan");
        assert_eq!(tet.day_can_chi.id, "giap_thin");
        assert_eq!(tet.hour_can_chi.id, "canh_ngo");
        assert!(!tet.hour_stem_day_shifted);

        // 2024-02-03 12:00 (before Lập Xuân): Quý Mão year, Ất Sửu BaZi month
        let pre = engine
            .full_can_chi(NaiveDate::from_ymd_opt(2024, 2, 3).unwrap(), 12, 0, 7.0)
            .unwrap();
        assert_eq!(pre.lunar_year_can_chi.id, "quy_mao");
        assert_eq!(pre.bazi_year_can_chi.id, "quy_mao");
        assert_eq!(pre.bazi_month_can_chi.id, "at_suu");

        // 2024-02-04 20:00 (after Lập Xuân ~15:27): Lunar still Quý Mão, BaZi switched to Giáp Thìn / Bính Dần
        let post = engine
            .full_can_chi(NaiveDate::from_ymd_opt(2024, 2, 4).unwrap(), 20, 0, 7.0)
            .unwrap();
        assert_eq!(post.lunar_year_can_chi.id, "quy_mao");
        assert_eq!(post.bazi_year_can_chi.id, "giap_thin");
        assert_eq!(post.bazi_month_can_chi.id, "binh_dan");
        assert_eq!(post.current_solar_term.id, "lap_xuan");

        // 2024-02-04 23:30: Mậu Tuất day, shifted to next day Kỷ Hợi stem -> Giáp Tý hour
        let late_ty = engine
            .full_can_chi(NaiveDate::from_ymd_opt(2024, 2, 4).unwrap(), 23, 30, 7.0)
            .unwrap();
        assert_eq!(late_ty.day_can_chi.id, "mau_tuat");
        assert_eq!(late_ty.hour_can_chi.id, "giap_ty");
        assert!(late_ty.hour_stem_day_shifted);
    }

    #[test]
    fn test_solar_terms_2024_and_boundaries() {
        let engine = make_engine();
        let terms = engine.solar_terms_for_year(2024, 7.0).unwrap();
        assert_eq!(terms.len(), 24);
        for w in terms.windows(2) {
            assert!(w[0].unix_seconds < w[1].unix_seconds);
        }
        let find_date = |id: &str| {
            terms
                .iter()
                .find(|t| t.id == id)
                .map(|t| t.date.clone())
                .unwrap()
        };
        assert_eq!(find_date("lap_xuan"), "2024-02-04");
        assert_eq!(find_date("xuan_phan"), "2024-03-20");
        assert_eq!(find_date("ha_chi"), "2024-06-21");
        assert_eq!(find_date("dong_chi"), "2024-12-21");

        // Month Can Chi across odd & even stems + wrapping months 11/12
        assert_eq!(engine.month_can_chi(0, 12).unwrap().id, "dinh_suu");
        assert_eq!(engine.month_can_chi(1, 12).unwrap().id, "ky_suu");
        assert!(engine.month_can_chi(0, 13).is_err());

        let d = NaiveDate::from_ymd_opt(2024, 2, 10).unwrap();
        assert!(engine.full_can_chi(d, 24, 0, 7.0).is_err());
        assert!(engine.full_can_chi(d, 12, 60, 7.0).is_err());
        assert!(engine.full_can_chi(d, 12, 0, 15.0).is_err());
        assert!(engine.full_can_chi(d, 12, 0, f64::NAN).is_err());
        assert!(engine.solar_terms_for_year(300_000, 7.0).is_err());
    }
}
