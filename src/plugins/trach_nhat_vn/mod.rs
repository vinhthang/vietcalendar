//! `TrachNhatPlugin`: Auspicious date & hour advisor (*Trạch Nhật*) over `CalendarService` and `CanChiSolarService`.

pub mod rules;

use std::sync::Arc;

use chrono::{Datelike, NaiveDate};
use serde_json::Value;

use crate::contracts::calendar::CalendarService;
use crate::contracts::canchi::CanChiSolarService;
use crate::contracts::models::CalendarError;
use crate::contracts::trach_nhat::{
    AuspiciousSearchQuery, AuspiciousSearchResponse, Occasion, TrachNhatDayAnalysis,
    TrachNhatService,
};
use crate::kernel::context::Context;
use crate::kernel::effect::BoxFuture;
use crate::kernel::error::KernelError;
use crate::kernel::fiber::{Plugin, PluginMeta};
use crate::plugins::calendar_vn::math::jd_from_date;
use crate::plugins::canchi_solar_vn::solar_terms::validate_timezone;
use crate::plugins::http_calendar::parse_ymd_ascii;
use crate::plugins::trach_nhat_vn::rules::evaluate_day;

const MAX_TRACH_NHAT_SEARCH_DAYS: i64 = 366;

/// Concrete [`TrachNhatService`] implementation.
pub struct TrachNhatEngine {
    #[allow(dead_code)]
    calendar: Arc<dyn CalendarService>,
    canchi: Arc<dyn CanChiSolarService>,
}

impl TrachNhatEngine {
    /// Builds a new [`TrachNhatEngine`] from injected calendar and Can Chi services.
    pub fn new(calendar: Arc<dyn CalendarService>, canchi: Arc<dyn CanChiSolarService>) -> Self {
        Self { calendar, canchi }
    }
}

fn parse_naive_date(raw: &str) -> Result<NaiveDate, CalendarError> {
    let (y, m, d) = parse_ymd_ascii(raw).ok_or_else(|| {
        CalendarError::InvalidSolarDate(format!("invalid date '{raw}'; expected YYYY-MM-DD"))
    })?;
    let month_u = u32::try_from(m).map_err(|_| CalendarError::InvalidSolarDate(raw.to_string()))?;
    let day_u = u32::try_from(d).map_err(|_| CalendarError::InvalidSolarDate(raw.to_string()))?;
    NaiveDate::from_ymd_opt(y, month_u, day_u)
        .ok_or_else(|| CalendarError::InvalidSolarDate(raw.to_string()))
}

impl TrachNhatService for TrachNhatEngine {
    fn analyze_date(
        &self,
        date: NaiveDate,
        occasion: Occasion,
        owner_birth_year: Option<i32>,
        partner_birth_year: Option<i32>,
        time_zone: f64,
    ) -> Result<TrachNhatDayAnalysis, CalendarError> {
        let (_, tz_hours) = validate_timezone(time_zone)?;
        let breakdown = self.canchi.full_can_chi(date, 12, 0, tz_hours)?;
        let jd = jd_from_date(date.day() as i32, date.month() as i32, date.year());
        evaluate_day(
            self.canchi.as_ref(),
            jd,
            &breakdown,
            occasion,
            owner_birth_year,
            partner_birth_year,
        )
    }

    fn find_auspicious_dates(
        &self,
        query: &AuspiciousSearchQuery,
    ) -> Result<AuspiciousSearchResponse, CalendarError> {
        let (_, tz_hours) = validate_timezone(query.time_zone)?;
        let start = parse_naive_date(&query.start_date)?;
        let end = parse_naive_date(&query.end_date)?;

        if end < start {
            return Err(CalendarError::InvalidSolarDate(
                "end_date must be on or after start_date".to_string(),
            ));
        }

        let span_days = (end - start).num_days() + 1;
        if span_days > MAX_TRACH_NHAT_SEARCH_DAYS {
            return Err(CalendarError::InvalidSolarDate(format!(
                "search range {span_days} days exceeds maximum {MAX_TRACH_NHAT_SEARCH_DAYS} days"
            )));
        }

        let min_score = query.min_score.unwrap_or(0);
        let limit = query.limit.unwrap_or(10).min(50);

        let mut candidates = Vec::with_capacity(span_days as usize);
        let mut cursor = start;
        while cursor <= end {
            let analysis = self.analyze_date(
                cursor,
                query.occasion,
                query.owner_birth_year,
                query.partner_birth_year,
                tz_hours,
            )?;
            if analysis.score >= min_score {
                candidates.push(analysis);
            }
            match cursor.succ_opt() {
                Some(next) => cursor = next,
                None => break,
            }
        }

        candidates.sort_by(|a, b| {
            b.score
                .cmp(&a.score)
                .then_with(|| a.solar_date.cmp(&b.solar_date))
        });
        candidates.truncate(limit);

        Ok(AuspiciousSearchResponse {
            occasion: query.occasion,
            start_date: query.start_date.clone(),
            end_date: query.end_date.clone(),
            evaluated_days: span_days as usize,
            candidates,
        })
    }
}

/// Cordis plugin providing [`TrachNhatService`].
pub struct TrachNhatPlugin;

impl Plugin for TrachNhatPlugin {
    fn meta(&self) -> PluginMeta {
        PluginMeta::new("trach_nhat_vn")
            .requires::<dyn CalendarService>()
            .requires::<dyn CanChiSolarService>()
            .provides::<dyn TrachNhatService>()
    }

    fn mount<'a>(&'a self, ctx: Context, _config: Value) -> BoxFuture<'a, Result<(), KernelError>> {
        Box::pin(async move {
            let calendar = ctx.inject::<dyn CalendarService>()?;
            let canchi = ctx.inject::<dyn CanChiSolarService>()?;
            let service: Arc<dyn TrachNhatService> =
                Arc::new(TrachNhatEngine::new(calendar, canchi));
            ctx.provide::<dyn TrachNhatService>(service);
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
    use crate::contracts::trach_nhat::{AuspiciousSearchQuery, Occasion, TrachNhatService};
    use crate::plugins::calendar_vn::VietnamCalendarEngine;
    use crate::plugins::canchi_solar_vn::CanChiSolarEngine;
    use crate::plugins::trach_nhat_vn::TrachNhatEngine;

    fn make_engine() -> TrachNhatEngine {
        let cal: Arc<dyn CalendarService> = Arc::new(VietnamCalendarEngine::new(256));
        let cc: Arc<dyn CanChiSolarService> = Arc::new(CanChiSolarEngine::new(Arc::clone(&cal)));
        TrachNhatEngine::new(cal, cc)
    }

    #[test]
    fn test_trach_nhat_2024_02_10_rules_and_age_checks() {
        let engine = make_engine();
        let d = NaiveDate::from_ymd_opt(2024, 2, 10).unwrap();
        let day = engine
            .analyze_date(d, Occasion::GrandOpening, Some(1990), None, 7.0)
            .unwrap();
        assert_eq!(day.truc.id, "man");
        assert_eq!(day.tu_star.id, "lieu");
        assert!(day.is_hoang_dao_day);
        assert_eq!(day.day_deity.id, "kim_quy");
        assert_eq!(day.hours.len(), 12);
        let hd_branches: Vec<&str> = day
            .hours
            .iter()
            .filter(|h| h.is_hoang_dao)
            .map(|h| h.branch.id.as_str())
            .collect();
        assert_eq!(
            hd_branches,
            vec!["dan", "thin", "ty_ran", "than", "dau", "hoi"]
        );

        let age = &day.age_checks[0];
        assert!(!age.tam_tai);
        assert!(!age.xung_tuoi_day);

        let d_2028 = NaiveDate::from_ymd_opt(2028, 3, 15).unwrap();
        let day_2028 = engine
            .analyze_date(d_2028, Occasion::Groundbreaking, Some(1990), None, 7.0)
            .unwrap();
        assert!(day_2028.age_checks[0].tam_tai);
    }

    #[test]
    fn test_trach_nhat_search_ranking_and_taboos() {
        let engine = make_engine();
        let tam_nuong = engine
            .analyze_date(
                NaiveDate::from_ymd_opt(2024, 2, 12).unwrap(),
                Occasion::Wedding,
                None,
                None,
                7.0,
            )
            .unwrap();
        assert!(tam_nuong.taboos.iter().any(|t| t.id == "tam_nuong"));

        let res = engine
            .find_auspicious_dates(&AuspiciousSearchQuery {
                start_date: "2024-02-10".to_string(),
                end_date: "2024-02-29".to_string(),
                occasion: Occasion::ContractSigning,
                owner_birth_year: Some(1990),
                partner_birth_year: None,
                min_score: Some(40),
                limit: Some(5),
                time_zone: 7.0,
            })
            .unwrap();
        assert_eq!(res.evaluated_days, 20);
        assert!(res.candidates.len() <= 5);
        for w in res.candidates.windows(2) {
            assert!(
                w[0].score > w[1].score
                    || (w[0].score == w[1].score && w[0].solar_date <= w[1].solar_date)
            );
        }
    }

    #[test]
    fn test_trach_nhat_range_and_birth_year_boundaries() {
        let engine = make_engine();
        assert!(engine
            .find_auspicious_dates(&AuspiciousSearchQuery {
                start_date: "2024-03-01".to_string(),
                end_date: "2024-02-01".to_string(),
                occasion: Occasion::General,
                owner_birth_year: None,
                partner_birth_year: None,
                min_score: None,
                limit: None,
                time_zone: 7.0,
            })
            .is_err());
        assert!(engine
            .find_auspicious_dates(&AuspiciousSearchQuery {
                start_date: "2024-01-01".to_string(),
                end_date: "2025-01-05".to_string(),
                occasion: Occasion::General,
                owner_birth_year: None,
                partner_birth_year: None,
                min_score: None,
                limit: None,
                time_zone: 7.0,
            })
            .is_err());
        assert!(engine
            .analyze_date(
                NaiveDate::from_ymd_opt(2024, 2, 10).unwrap(),
                Occasion::Wedding,
                Some(0),
                None,
                7.0,
            )
            .is_err());
    }
}
