//! `AstrologyPlugin`: Deterministic Vietnamese Tử Vi (Nam Phái Core) & BaZi Four Pillars engine.

pub mod bazi;
pub mod tuvi;

use std::sync::Arc;

use chrono::NaiveDate;
use serde_json::Value;

use crate::contracts::astrology::{
    AstrologyChartResponse, AstrologyQuery, AstrologyService, ChartSystem,
};
use crate::contracts::calendar::CalendarService;
use crate::contracts::canchi::CanChiSolarService;
use crate::contracts::models::CalendarError;
use crate::kernel::context::Context;
use crate::kernel::effect::BoxFuture;
use crate::kernel::error::KernelError;
use crate::kernel::fiber::{Plugin, PluginMeta};
use crate::plugins::astrology_vn::bazi::build_bazi_chart;
use crate::plugins::astrology_vn::tuvi::build_tuvi_chart;
use crate::plugins::canchi_solar_vn::solar_terms::{validate_hour_minute, validate_timezone};
use crate::plugins::http_calendar::parse_ymd_ascii;

/// Concrete [`AstrologyService`] implementation backed by `CalendarService` and `CanChiSolarService`.
pub struct AstrologyEngine {
    calendar: Arc<dyn CalendarService>,
    canchi: Arc<dyn CanChiSolarService>,
}

impl AstrologyEngine {
    /// Builds a new [`AstrologyEngine`] from injected calendar and Can Chi services.
    pub fn new(calendar: Arc<dyn CalendarService>, canchi: Arc<dyn CanChiSolarService>) -> Self {
        Self { calendar, canchi }
    }
}

impl AstrologyService for AstrologyEngine {
    fn generate_chart(
        &self,
        query: &AstrologyQuery,
    ) -> Result<AstrologyChartResponse, CalendarError> {
        let (_, tz_hours) = validate_timezone(query.time_zone)?;
        validate_hour_minute(query.hour, query.minute)?;

        let (y, m, d) = parse_ymd_ascii(&query.date).ok_or_else(|| {
            CalendarError::InvalidSolarDate(format!(
                "invalid date '{}'; expected YYYY-MM-DD",
                query.date
            ))
        })?;

        let solar_date = if query.is_lunar {
            self.calendar
                .to_solar(d, m, y, query.is_leap_month, tz_hours)?
        } else {
            NaiveDate::from_ymd_opt(
                y,
                u32::try_from(m)
                    .map_err(|_| CalendarError::InvalidSolarDate(query.date.clone()))?,
                u32::try_from(d)
                    .map_err(|_| CalendarError::InvalidSolarDate(query.date.clone()))?,
            )
            .ok_or_else(|| CalendarError::InvalidSolarDate(query.date.clone()))?
        };

        let breakdown = self
            .canchi
            .full_can_chi(solar_date, query.hour, query.minute, tz_hours)?;

        let bazi = match query.system {
            ChartSystem::Bazi | ChartSystem::Both => Some(build_bazi_chart(
                self.canchi.as_ref(),
                solar_date,
                query.gender,
                &breakdown,
            )?),
            ChartSystem::Tuvi => None,
        };

        let tuvi = match query.system {
            ChartSystem::Tuvi | ChartSystem::Both => Some(build_tuvi_chart(
                self.canchi.as_ref(),
                query.gender,
                &breakdown,
            )?),
            ChartSystem::Bazi => None,
        };

        Ok(AstrologyChartResponse {
            solar_date: solar_date.format("%Y-%m-%d").to_string(),
            lunar: breakdown.lunar.clone(),
            hour: query.hour,
            minute: query.minute,
            gender: query.gender,
            system: query.system,
            can_chi: breakdown,
            bazi,
            tuvi,
        })
    }
}

/// Cordis plugin providing [`AstrologyService`].
pub struct AstrologyPlugin;

impl Plugin for AstrologyPlugin {
    fn meta(&self) -> PluginMeta {
        PluginMeta::new("astrology_vn")
            .requires::<dyn CalendarService>()
            .requires::<dyn CanChiSolarService>()
            .provides::<dyn AstrologyService>()
    }

    fn mount<'a>(&'a self, ctx: Context, _config: Value) -> BoxFuture<'a, Result<(), KernelError>> {
        Box::pin(async move {
            let calendar = ctx.inject::<dyn CalendarService>()?;
            let canchi = ctx.inject::<dyn CanChiSolarService>()?;
            let service: Arc<dyn AstrologyService> =
                Arc::new(AstrologyEngine::new(calendar, canchi));
            ctx.provide::<dyn AstrologyService>(service);
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::sync::Arc;

    use crate::contracts::astrology::{AstrologyQuery, AstrologyService, ChartSystem, Gender};
    use crate::contracts::calendar::CalendarService;
    use crate::contracts::canchi::CanChiSolarService;
    use crate::plugins::astrology_vn::AstrologyEngine;
    use crate::plugins::calendar_vn::VietnamCalendarEngine;
    use crate::plugins::canchi_solar_vn::CanChiSolarEngine;

    fn make_engine() -> (Arc<dyn CalendarService>, AstrologyEngine) {
        let cal: Arc<dyn CalendarService> = Arc::new(VietnamCalendarEngine::new(256));
        let cc: Arc<dyn CanChiSolarService> = Arc::new(CanChiSolarEngine::new(Arc::clone(&cal)));
        let ast = AstrologyEngine::new(Arc::clone(&cal), cc);
        (cal, ast)
    }

    #[test]
    fn test_bazi_lap_xuan_split_and_luck_direction() {
        let (_, ast) = make_engine();
        let pre = ast
            .generate_chart(&AstrologyQuery {
                date: "2024-02-03".to_string(),
                hour: 12,
                minute: 0,
                gender: Gender::Male,
                system: ChartSystem::Bazi,
                is_lunar: false,
                is_leap_month: false,
                time_zone: 7.0,
            })
            .unwrap();
        let bazi_pre = pre.bazi.unwrap();
        assert_eq!(bazi_pre.year_pillar.can_chi.id, "quy_mao");
        assert_eq!(bazi_pre.month_pillar.can_chi.id, "at_suu");
        assert_eq!(bazi_pre.day_pillar.ten_god_stem.id, "nhat_chu");
        assert_eq!(bazi_pre.day_master_element.id, "hoa");
        assert_eq!(bazi_pre.year_pillar.hidden_stems[0].stem.id, "at");
        assert_eq!(bazi_pre.year_pillar.hidden_stems[0].element.id, "moc");
        assert!(!bazi_pre.forward_luck_direction);
        assert_eq!(bazi_pre.luck_cycles.len(), 8);

        let post_male = ast
            .generate_chart(&AstrologyQuery {
                date: "2024-02-04".to_string(),
                hour: 20,
                minute: 0,
                gender: Gender::Male,
                system: ChartSystem::Bazi,
                is_lunar: false,
                is_leap_month: false,
                time_zone: 7.0,
            })
            .unwrap();
        let b_male = post_male.bazi.unwrap();
        assert_eq!(b_male.year_pillar.can_chi.id, "giap_thin");
        assert_eq!(b_male.month_pillar.can_chi.id, "binh_dan");
        assert!(b_male.forward_luck_direction);
        assert_eq!(b_male.luck_cycles[0].can_chi.id, "dinh_mao");
        assert!(!b_male.year_pillar.hidden_stems.is_empty());

        let post_female = ast
            .generate_chart(&AstrologyQuery {
                date: "2024-02-04".to_string(),
                hour: 20,
                minute: 0,
                gender: Gender::Female,
                system: ChartSystem::Bazi,
                is_lunar: false,
                is_leap_month: false,
                time_zone: 7.0,
            })
            .unwrap();
        let b_fem = post_female.bazi.unwrap();
        assert!(!b_fem.forward_luck_direction);
        assert_eq!(b_fem.luck_cycles[0].can_chi.id, "at_suu");
    }

    #[test]
    fn test_tuvi_chart_invariants_cuc_stars_and_leap_month() {
        let (_, ast) = make_engine();
        // Lunar 21/04/1990 (Canh Ngo), hour 10 (Ty branch=5) -> Menh at Ty (0), Mau Ty -> Hoa Luc Cuc (6)
        let res = ast
            .generate_chart(&AstrologyQuery {
                date: "1990-05-15".to_string(),
                hour: 10,
                minute: 0,
                gender: Gender::Male,
                system: ChartSystem::Both,
                is_lunar: false,
                is_leap_month: false,
                time_zone: 7.0,
            })
            .unwrap();
        let tv = res.tuvi.unwrap();
        assert_eq!(tv.palaces.len(), 12);
        assert_eq!(tv.menh_branch_index, 0);
        assert_eq!(tv.than_branch_index, 10);
        assert_eq!(tv.cuc_number, 6);
        assert_eq!(tv.tu_hoa_summary.len(), 4);
        assert_eq!(
            tv.thien_phu_branch_index,
            (4 - i32::from(tv.tu_vi_branch_index)).rem_euclid(12) as u8
        );
        assert!(!tv.leap_month_shifted_for_tuvi);

        let menh_count = tv.palaces.iter().filter(|p| p.is_menh).count();
        let than_count = tv.palaces.iter().filter(|p| p.is_than).count();
        assert_eq!(menh_count, 1);
        assert_eq!(than_count, 1);

        let mut star_ids = HashSet::new();
        for p in &tv.palaces {
            for s in &p.stars {
                star_ids.insert(s.id.as_str());
            }
        }
        assert!(star_ids.contains("tu_vi"));
        assert!(star_ids.contains("thien_phu"));
        assert!(star_ids.contains("loc_ton"));
        assert!(star_ids.contains("thien_ma"));
        assert!(star_ids.contains("thai_tue"));
        assert!(star_ids.len() >= 45, "got {} unique stars", star_ids.len());

        // 2025 leap month 6: day 5 stays on month 6, day 20 shifts for Tu Vi
        let leap_early = ast
            .generate_chart(&AstrologyQuery {
                date: "2025-06-05".to_string(),
                hour: 9,
                minute: 0,
                gender: Gender::Female,
                system: ChartSystem::Tuvi,
                is_lunar: true,
                is_leap_month: true,
                time_zone: 7.0,
            })
            .unwrap()
            .tuvi
            .unwrap();
        assert!(!leap_early.leap_month_shifted_for_tuvi);

        let leap_late = ast
            .generate_chart(&AstrologyQuery {
                date: "2025-06-20".to_string(),
                hour: 9,
                minute: 0,
                gender: Gender::Female,
                system: ChartSystem::Tuvi,
                is_lunar: true,
                is_leap_month: true,
                time_zone: 7.0,
            })
            .unwrap()
            .tuvi
            .unwrap();
        assert!(leap_late.leap_month_shifted_for_tuvi);
    }

    #[test]
    fn test_astrology_invalid_input_errors() {
        let (_, ast) = make_engine();
        let bad_cases = [
            AstrologyQuery {
                date: "not-a-date".to_string(),
                hour: 10,
                minute: 0,
                gender: Gender::Male,
                system: ChartSystem::Both,
                is_lunar: false,
                is_leap_month: false,
                time_zone: 7.0,
            },
            AstrologyQuery {
                date: "1990-05-15".to_string(),
                hour: 24,
                minute: 0,
                gender: Gender::Male,
                system: ChartSystem::Bazi,
                is_lunar: false,
                is_leap_month: false,
                time_zone: 7.0,
            },
            AstrologyQuery {
                date: "1990-05-15".to_string(),
                hour: 10,
                minute: 60,
                gender: Gender::Female,
                system: ChartSystem::Tuvi,
                is_lunar: false,
                is_leap_month: false,
                time_zone: 7.0,
            },
            AstrologyQuery {
                date: "2024-06-15".to_string(),
                hour: 10,
                minute: 0,
                gender: Gender::Female,
                system: ChartSystem::Tuvi,
                is_lunar: true,
                is_leap_month: true,
                time_zone: 7.0,
            },
        ];
        for q in &bad_cases {
            assert!(ast.generate_chart(q).is_err());
        }
    }
}
