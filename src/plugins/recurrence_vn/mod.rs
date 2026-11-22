//! `LunarRecurrencePlugin`: Stateless multi-year lunar recurrence & RFC 5545 `.ics` exporter.

pub mod ics;

use std::collections::HashSet;
use std::sync::Arc;

use chrono::{Datelike, NaiveDate};
use serde_json::Value;

use crate::contracts::calendar::CalendarService;
use crate::contracts::canchi::BilingualLabel;
use crate::contracts::models::CalendarError;
use crate::contracts::recurrence::{
    Day30Policy, LeapMonthPolicy, LunarRecurrenceService, RecurrenceFrequency,
    RecurrenceOccurrence, RecurrencePreset, RecurrenceRequest, RecurrenceResponse,
};
use crate::kernel::context::Context;
use crate::kernel::effect::BoxFuture;
use crate::kernel::error::KernelError;
use crate::kernel::fiber::{Plugin, PluginMeta};
use crate::plugins::canchi_solar_vn::solar_terms::validate_timezone;
use crate::plugins::http_calendar::parse_ymd_ascii;
use crate::plugins::recurrence_vn::ics::render_ics;

const MAX_RECURRENCE_SPAN_DAYS: i64 = 18_263; // 50 years inclusive of leap days

/// Concrete stateless [`LunarRecurrenceService`] implementation.
pub struct LunarRecurrenceEngine {
    calendar: Arc<dyn CalendarService>,
}

impl LunarRecurrenceEngine {
    /// Builds a new [`LunarRecurrenceEngine`] over an existing [`CalendarService`].
    pub fn new(calendar: Arc<dyn CalendarService>) -> Self {
        Self { calendar }
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

struct EffectiveRule {
    title: String,
    frequency: RecurrenceFrequency,
    target_days: Vec<i32>,
    target_month: Option<i32>,
}

fn resolve_rule(req: &RecurrenceRequest) -> Result<EffectiveRule, CalendarError> {
    if let Some(preset) = req.preset {
        let (default_title, days) = match preset {
            RecurrencePreset::NewAndFullMoon => ("Mùng 1 & Rằm Âm lịch", vec![1, 15]),
            RecurrencePreset::VegetarianFourDays => ("Ăn chay Tứ Trai", vec![1, 14, 15, 30]),
            RecurrencePreset::VegetarianSixDays => {
                ("Ăn chay Lục Trai", vec![8, 14, 15, 23, 29, 30])
            }
            RecurrencePreset::VegetarianTenDays => (
                "Ăn chay Thập Trai",
                vec![1, 8, 14, 15, 18, 23, 24, 28, 29, 30],
            ),
        };
        return Ok(EffectiveRule {
            title: req
                .title
                .clone()
                .unwrap_or_else(|| default_title.to_string()),
            frequency: RecurrenceFrequency::Monthly,
            target_days: days,
            target_month: None,
        });
    }

    let frequency = req.frequency.ok_or_else(|| {
        CalendarError::InvalidSolarDate("either preset or frequency must be provided".to_string())
    })?;
    let lunar_day = req.lunar_day.ok_or_else(|| {
        CalendarError::InvalidSolarDate("lunar_day is required when preset is omitted".to_string())
    })?;
    if !(1..=30).contains(&lunar_day) {
        return Err(CalendarError::InvalidLunarDay {
            day: lunar_day,
            month: req.lunar_month.unwrap_or(1),
            year: 2024,
            max_days: 30,
        });
    }

    let target_month = match frequency {
        RecurrenceFrequency::Monthly => {
            if let Some(m) = req.lunar_month {
                if !(1..=12).contains(&m) {
                    return Err(CalendarError::InvalidLunarMonth(m));
                }
            }
            None
        }
        RecurrenceFrequency::Yearly => {
            let m = req.lunar_month.ok_or(CalendarError::InvalidLunarMonth(0))?;
            if !(1..=12).contains(&m) {
                return Err(CalendarError::InvalidLunarMonth(m));
            }
            Some(m)
        }
    };

    let title = req.title.clone().unwrap_or_else(|| match target_month {
        Some(m) => format!("Sự kiện Âm lịch {lunar_day:02}/{m:02}"),
        None => format!("Sự kiện Âm lịch mùng {lunar_day:02} hằng tháng"),
    });

    Ok(EffectiveRule {
        title,
        frequency,
        target_days: vec![lunar_day],
        target_month,
    })
}

fn label_for_occurrence(day: i32, month: i32, is_leap: bool) -> BilingualLabel {
    let slug_leap = if is_leap { "_nhuan" } else { "" };
    let vi_leap = if is_leap { " (nhuận)" } else { "" };
    let en_leap = if is_leap { " (Leap)" } else { "" };
    BilingualLabel::new(
        format!("lunar_{day:02}_{month:02}{slug_leap}"),
        format!("Ngày {day:02}/{month:02}{vi_leap} Âm lịch"),
        format!("Lunar Day {day:02}/{month:02}{en_leap}"),
    )
}

impl LunarRecurrenceService for LunarRecurrenceEngine {
    fn expand_occurrences(
        &self,
        req: &RecurrenceRequest,
    ) -> Result<RecurrenceResponse, CalendarError> {
        let (_, tz_hours) = validate_timezone(req.time_zone)?;
        let start = parse_naive_date(&req.start_date)?;
        let end = parse_naive_date(&req.end_date)?;

        if end < start {
            return Err(CalendarError::InvalidSolarDate(
                "end_date must be on or after start_date".to_string(),
            ));
        }

        let span_days = (end - start).num_days();
        if span_days > MAX_RECURRENCE_SPAN_DAYS {
            return Err(CalendarError::InvalidSolarDate(format!(
                "recurrence span {span_days} days exceeds maximum 50-year horizon"
            )));
        }

        let rule = resolve_rule(req)?;
        let start_lunar = self.calendar.to_lunar(start, tz_hours)?;
        let end_lunar = self.calendar.to_lunar(end, tz_hours)?;

        let mut occurrences = Vec::new();
        let mut seen_solar: HashSet<NaiveDate> = HashSet::new();

        for lunar_year in start_lunar.year..=end_lunar.year {
            let has_leap_year = self.calendar.is_lunar_leap_year(lunar_year, tz_hours)?;
            let months: Vec<i32> = match rule.frequency {
                RecurrenceFrequency::Yearly => rule.target_month.into_iter().collect(),
                RecurrenceFrequency::Monthly => (1..=12).collect(),
            };

            for month in months {
                let mut variants = Vec::with_capacity(2);
                if matches!(
                    req.leap_month_policy,
                    LeapMonthPolicy::PrimaryOnly | LeapMonthPolicy::IncludeLeap
                ) {
                    variants.push(false);
                }
                if has_leap_year
                    && matches!(
                        req.leap_month_policy,
                        LeapMonthPolicy::IncludeLeap | LeapMonthPolicy::LeapOnly
                    )
                    && self
                        .calendar
                        .lunar_month_days(month, lunar_year, true, tz_hours)
                        .is_ok()
                {
                    variants.push(true);
                }

                for is_leap in variants {
                    let month_len = self
                        .calendar
                        .lunar_month_days(month, lunar_year, is_leap, tz_hours)?;
                    for &req_day in &rule.target_days {
                        let (resolved_day, adjusted) = if req_day <= i32::from(month_len) {
                            (req_day, false)
                        } else if req_day == 30
                            && month_len == 29
                            && req.day30_policy == Day30Policy::LastDayOfMonth
                        {
                            (29, true)
                        } else {
                            continue;
                        };

                        let solar = self.calendar.to_solar(
                            resolved_day,
                            month,
                            lunar_year,
                            is_leap,
                            tz_hours,
                        )?;
                        if solar < start || solar > end || !seen_solar.insert(solar) {
                            continue;
                        }

                        occurrences.push(RecurrenceOccurrence {
                            solar_date: solar.format("%Y-%m-%d").to_string(),
                            lunar: crate::contracts::models::LunarDate {
                                day: resolved_day,
                                month,
                                year: lunar_year,
                                is_leap,
                            },
                            label: label_for_occurrence(resolved_day, month, is_leap),
                            adjusted_from_day30: adjusted,
                            is_leap_month_occurrence: is_leap,
                        });
                    }
                }
            }
        }

        occurrences.sort_by(|a, b| a.solar_date.cmp(&b.solar_date));
        // Validate year bounds in final dates stayed consistent (also avoids unused Datelike lint if any).
        let _ = start.year();

        Ok(RecurrenceResponse {
            title: rule.title,
            start_date: req.start_date.clone(),
            end_date: req.end_date.clone(),
            day30_policy: req.day30_policy,
            leap_month_policy: req.leap_month_policy,
            count: occurrences.len(),
            occurrences,
        })
    }

    fn export_ics(&self, req: &RecurrenceRequest) -> Result<String, CalendarError> {
        let resp = self.expand_occurrences(req)?;
        Ok(render_ics(req, &resp))
    }
}

/// Cordis plugin providing [`LunarRecurrenceService`].
pub struct LunarRecurrencePlugin;

impl Plugin for LunarRecurrencePlugin {
    fn meta(&self) -> PluginMeta {
        PluginMeta::new("recurrence_vn")
            .requires::<dyn CalendarService>()
            .provides::<dyn LunarRecurrenceService>()
    }

    fn mount<'a>(&'a self, ctx: Context, _config: Value) -> BoxFuture<'a, Result<(), KernelError>> {
        Box::pin(async move {
            let calendar = ctx.inject::<dyn CalendarService>()?;
            let service: Arc<dyn LunarRecurrenceService> =
                Arc::new(LunarRecurrenceEngine::new(calendar));
            ctx.provide::<dyn LunarRecurrenceService>(service);
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::contracts::calendar::CalendarService;
    use crate::contracts::recurrence::{
        Day30Policy, LeapMonthPolicy, LunarRecurrenceService, RecurrenceFrequency,
        RecurrencePreset, RecurrenceRequest,
    };
    use crate::plugins::calendar_vn::VietnamCalendarEngine;
    use crate::plugins::recurrence_vn::LunarRecurrenceEngine;

    fn make_engine() -> LunarRecurrenceEngine {
        let cal: Arc<dyn CalendarService> = Arc::new(VietnamCalendarEngine::new(256));
        LunarRecurrenceEngine::new(cal)
    }

    #[test]
    fn test_recurrence_day30_and_leap_month_policies() {
        let engine = make_engine();

        let base_day30 = RecurrenceRequest {
            title: Some("Tất Niên / Cuối tháng".to_string()),
            description: None,
            frequency: Some(RecurrenceFrequency::Monthly),
            preset: None,
            lunar_day: Some(30),
            lunar_month: None,
            start_date: "2025-01-01".to_string(),
            end_date: "2025-06-30".to_string(),
            day30_policy: Day30Policy::Skip,
            leap_month_policy: LeapMonthPolicy::PrimaryOnly,
            time_zone: 7.0,
            reminder_minutes: None,
        };
        let skipped = engine.expand_occurrences(&base_day30).unwrap();
        assert!(!skipped
            .occurrences
            .iter()
            .any(|o| o.solar_date == "2025-01-28"));

        let mut last_day = base_day30;
        last_day.day30_policy = Day30Policy::LastDayOfMonth;
        let resolved = engine.expand_occurrences(&last_day).unwrap();
        let jan28 = resolved
            .occurrences
            .iter()
            .find(|o| o.solar_date == "2025-01-28")
            .unwrap();
        assert!(jan28.adjusted_from_day30);
        assert_eq!(jan28.lunar.day, 29);

        let leap_req = |policy: LeapMonthPolicy| RecurrenceRequest {
            title: Some("Rằm Tháng Hai".to_string()),
            description: None,
            frequency: Some(RecurrenceFrequency::Yearly),
            preset: None,
            lunar_day: Some(15),
            lunar_month: Some(2),
            start_date: "2023-01-01".to_string(),
            end_date: "2023-12-31".to_string(),
            day30_policy: Day30Policy::LastDayOfMonth,
            leap_month_policy: policy,
            time_zone: 7.0,
            reminder_minutes: None,
        };
        assert_eq!(
            engine
                .expand_occurrences(&leap_req(LeapMonthPolicy::PrimaryOnly))
                .unwrap()
                .count,
            1
        );
        let inc = engine
            .expand_occurrences(&leap_req(LeapMonthPolicy::IncludeLeap))
            .unwrap();
        assert_eq!(inc.count, 2);
        let leap_only = engine
            .expand_occurrences(&leap_req(LeapMonthPolicy::LeapOnly))
            .unwrap();
        assert_eq!(leap_only.count, 1);
        assert!(leap_only.occurrences[0].is_leap_month_occurrence);

        // Inclusive start/end date boundary + exact single-day span (start == end)
        let exact_bound = RecurrenceRequest {
            title: Some("Boundary check".to_string()),
            description: None,
            frequency: None,
            preset: Some(RecurrencePreset::NewAndFullMoon),
            lunar_day: None,
            lunar_month: None,
            start_date: "2025-01-29".to_string(),
            end_date: "2025-02-12".to_string(),
            day30_policy: Day30Policy::Skip,
            leap_month_policy: LeapMonthPolicy::PrimaryOnly,
            time_zone: 7.0,
            reminder_minutes: None,
        };
        let eb = engine.expand_occurrences(&exact_bound).unwrap();
        assert_eq!(eb.count, 2);
        assert_eq!(eb.occurrences[0].solar_date, "2025-01-29");
        assert_eq!(eb.occurrences[1].solar_date, "2025-02-12");

        let single_day = RecurrenceRequest {
            end_date: "2025-01-29".to_string(),
            ..exact_bound
        };
        assert_eq!(engine.expand_occurrences(&single_day).unwrap().count, 1);
    }

    #[test]
    fn test_recurrence_presets_and_ics_folding() {
        let engine = make_engine();
        let req = RecurrenceRequest {
            title: Some("Lịch Ăn Chay Tứ Trai Truyền Thống Gia Đình Việt Nam (Nhắc nhở tự động hằng tháng)".to_string()),
            description: Some("Sự kiện lặp theo Âm lịch Việt Nam UTC+7, tự động điều chỉnh tháng thiếu 29 ngày và tháng nhuận.".to_string()),
            frequency: None,
            preset: Some(RecurrencePreset::VegetarianFourDays),
            lunar_day: None,
            lunar_month: None,
            start_date: "2025-01-25".to_string(),
            end_date: "2025-03-25".to_string(),
            day30_policy: Day30Policy::LastDayOfMonth,
            leap_month_policy: LeapMonthPolicy::IncludeLeap,
            time_zone: 7.0,
            reminder_minutes: Some(720),
        };
        let resp = engine.expand_occurrences(&req).unwrap();
        assert!(resp.count >= 7);

        let ics = engine.export_ics(&req).unwrap();
        assert!(ics.starts_with("BEGIN:VCALENDAR\r\n"));
        assert!(ics.ends_with("END:VCALENDAR\r\n"));
        assert!(ics.contains("BEGIN:VALARM\r\n"));
        for line in ics.split("\r\n") {
            assert!(
                line.len() <= 75,
                "ICS line exceeds 75 octets ({} bytes): {line}",
                line.len()
            );
        }
        assert!(ics.contains("\r\n "));
    }

    #[test]
    fn test_recurrence_validation_boundaries() {
        let engine = make_engine();
        let invalid = [
            RecurrenceRequest {
                title: None,
                description: None,
                frequency: Some(RecurrenceFrequency::Monthly),
                preset: None,
                lunar_day: Some(31),
                lunar_month: None,
                start_date: "2025-01-01".to_string(),
                end_date: "2025-12-31".to_string(),
                day30_policy: Day30Policy::Skip,
                leap_month_policy: LeapMonthPolicy::PrimaryOnly,
                time_zone: 7.0,
                reminder_minutes: None,
            },
            RecurrenceRequest {
                title: None,
                description: None,
                frequency: Some(RecurrenceFrequency::Yearly),
                preset: None,
                lunar_day: Some(15),
                lunar_month: Some(13),
                start_date: "2025-01-01".to_string(),
                end_date: "2025-12-31".to_string(),
                day30_policy: Day30Policy::Skip,
                leap_month_policy: LeapMonthPolicy::PrimaryOnly,
                time_zone: 7.0,
                reminder_minutes: None,
            },
            RecurrenceRequest {
                title: None,
                description: None,
                frequency: Some(RecurrenceFrequency::Monthly),
                preset: None,
                lunar_day: Some(1),
                lunar_month: None,
                start_date: "2020-01-01".to_string(),
                end_date: "2075-01-01".to_string(),
                day30_policy: Day30Policy::Skip,
                leap_month_policy: LeapMonthPolicy::PrimaryOnly,
                time_zone: 7.0,
                reminder_minutes: None,
            },
            RecurrenceRequest {
                title: None,
                description: None,
                frequency: Some(RecurrenceFrequency::Monthly),
                preset: None,
                lunar_day: Some(15),
                lunar_month: Some(13),
                start_date: "2025-01-01".to_string(),
                end_date: "2025-12-31".to_string(),
                day30_policy: Day30Policy::Skip,
                leap_month_policy: LeapMonthPolicy::PrimaryOnly,
                time_zone: 7.0,
                reminder_minutes: None,
            },
            RecurrenceRequest {
                title: None,
                description: None,
                frequency: Some(RecurrenceFrequency::Monthly),
                preset: None,
                lunar_day: Some(1),
                lunar_month: None,
                start_date: "2025-02-01".to_string(),
                end_date: "2025-01-01".to_string(),
                day30_policy: Day30Policy::Skip,
                leap_month_policy: LeapMonthPolicy::PrimaryOnly,
                time_zone: 7.0,
                reminder_minutes: None,
            },
            RecurrenceRequest {
                title: None,
                description: None,
                frequency: None,
                preset: None,
                lunar_day: Some(1),
                lunar_month: None,
                start_date: "2025-01-01".to_string(),
                end_date: "2025-06-01".to_string(),
                day30_policy: Day30Policy::Skip,
                leap_month_policy: LeapMonthPolicy::PrimaryOnly,
                time_zone: 7.0,
                reminder_minutes: None,
            },
        ];
        for r in &invalid {
            assert!(engine.expand_occurrences(r).is_err());
        }

        // Exact max 50-year span (18263 days: 2000-01-01 ..= 2050-01-01) succeeds; +1 day fails
        let exact_50yr = RecurrenceRequest {
            title: None,
            description: None,
            frequency: Some(RecurrenceFrequency::Yearly),
            preset: None,
            lunar_day: Some(1),
            lunar_month: Some(1),
            start_date: "2000-01-01".to_string(),
            end_date: "2050-01-01".to_string(),
            day30_policy: Day30Policy::Skip,
            leap_month_policy: LeapMonthPolicy::PrimaryOnly,
            time_zone: 7.0,
            reminder_minutes: None,
        };
        assert!(engine.expand_occurrences(&exact_50yr).is_ok());
        let over_50yr = RecurrenceRequest {
            end_date: "2050-01-02".to_string(),
            ..exact_50yr
        };
        assert!(engine.expand_occurrences(&over_50yr).is_err());
    }
}
