//! `RetailCalendarPlugin`: Wait-free hot-reloadable Vietnam lunar retail & seasonal peak engine.

pub mod loader;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use arc_swap::ArcSwap;
use chrono::{Datelike, Duration, NaiveDate};
use serde::Deserialize;
use serde_json::Value;

use crate::contracts::calendar::CalendarService;
use crate::contracts::models::{CalendarError, LunarDate};
use crate::contracts::recurrence::{Day30Policy, LeapMonthPolicy};
use crate::contracts::retail::{
    RetailCalendarService, RetailCampaignMilestone, RetailCatalog, RetailCategory,
    RetailEventOccurrence, RetailUpcomingResponse, RetailYearResponse,
};
use crate::kernel::context::Context;
use crate::kernel::effect::BoxFuture;
use crate::kernel::error::KernelError;
use crate::kernel::fiber::{Plugin, PluginMeta};
use crate::plugins::canchi_solar_vn::solar_terms::{validate_timezone, validate_year_bounds};
use crate::plugins::retail_vn::loader::load_retail_catalog;

const MAX_RETAIL_HORIZON_DAYS: i32 = 366;

#[derive(Debug, Clone, Deserialize)]
struct RetailConfig {
    #[serde(default = "default_retail_file")]
    catalog_path: String,
    #[serde(default)]
    config_base_dir: Option<String>,
}

fn default_retail_file() -> String {
    "retail_events.yml".to_string()
}

fn resolve_catalog_path(cfg: &RetailConfig) -> PathBuf {
    let p = Path::new(&cfg.catalog_path);
    if p.is_absolute() {
        return p.to_path_buf();
    }
    if let Some(ref base) = cfg.config_base_dir {
        return PathBuf::from(base).join(p);
    }
    if let Ok(env_dir) = std::env::var("VIETCALENDAR_CONFIG_DIR") {
        return PathBuf::from(env_dir).join(p);
    }
    let under_config = Path::new("config").join(p);
    if under_config.exists() {
        return under_config;
    }
    p.to_path_buf()
}

/// Concrete wait-free [`RetailCalendarService`] implementation backed by `ArcSwap<RetailCatalog>`.
pub struct RetailCalendarEngine {
    calendar: Arc<dyn CalendarService>,
    catalog: ArcSwap<RetailCatalog>,
}

impl RetailCalendarEngine {
    /// Creates a new retail calendar engine with an initial validated catalog.
    pub fn new(calendar: Arc<dyn CalendarService>, catalog: Arc<RetailCatalog>) -> Self {
        Self {
            calendar,
            catalog: ArcSwap::new(catalog),
        }
    }

    fn resolve_single_lunar_occurrence(
        &self,
        day: i32,
        month: i32,
        lunar_year: i32,
        is_leap: bool,
        day30_policy: Day30Policy,
        tz_hours: f64,
    ) -> Result<Option<(NaiveDate, LunarDate)>, CalendarError> {
        let month_len = self
            .calendar
            .lunar_month_days(month, lunar_year, is_leap, tz_hours)?;
        let resolved_day = if day <= i32::from(month_len) {
            day
        } else if day == 30 && month_len == 29 && day30_policy == Day30Policy::LastDayOfMonth {
            29
        } else {
            return Ok(None);
        };
        let solar = self
            .calendar
            .to_solar(resolved_day, month, lunar_year, is_leap, tz_hours)?;
        Ok(Some((
            solar,
            LunarDate {
                day: resolved_day,
                month,
                year: lunar_year,
                is_leap,
            },
        )))
    }

    fn collect_for_window(
        &self,
        start_solar: NaiveDate,
        end_exclusive: NaiveDate,
        category: Option<RetailCategory>,
        tz_hours: f64,
    ) -> Result<Vec<RetailEventOccurrence>, CalendarError> {
        let catalog = self.catalog.load();
        let start_lunar = self.calendar.to_lunar(start_solar, tz_hours)?;
        let end_lunar = self
            .calendar
            .to_lunar(end_exclusive.pred_opt().unwrap_or(end_exclusive), tz_hours)?;

        let mut results = Vec::new();
        for lunar_year in (start_lunar.year - 1)..=(end_lunar.year + 1) {
            let has_leap = self.calendar.is_lunar_leap_year(lunar_year, tz_hours)?;

            // 1. Declarative catalog events
            for rule in &catalog.events {
                if let Some(cat) = category {
                    if !rule.categories.contains(&cat) {
                        continue;
                    }
                }
                let mut leap_flags = Vec::with_capacity(2);
                if matches!(
                    rule.leap_month_policy,
                    LeapMonthPolicy::PrimaryOnly | LeapMonthPolicy::IncludeLeap
                ) {
                    leap_flags.push(false);
                }
                if has_leap
                    && matches!(
                        rule.leap_month_policy,
                        LeapMonthPolicy::IncludeLeap | LeapMonthPolicy::LeapOnly
                    )
                    && self
                        .calendar
                        .lunar_month_days(rule.lunar_month, lunar_year, true, tz_hours)
                        .is_ok()
                {
                    leap_flags.push(true);
                }

                for is_leap in leap_flags {
                    if let Some((peak, lunar_dt)) = self.resolve_single_lunar_occurrence(
                        rule.lunar_day,
                        rule.lunar_month,
                        lunar_year,
                        is_leap,
                        rule.day30_policy,
                        tz_hours,
                    )? {
                        if peak < start_solar || peak >= end_exclusive {
                            continue;
                        }
                        let prep = peak - Duration::days(rule.prep_lead_days);
                        let early = peak - Duration::days(rule.early_bird_lead_days);
                        results.push(RetailEventOccurrence {
                            id: rule.id.clone(),
                            name_vi: rule.name_vi.clone(),
                            name_en: rule.name_en.clone(),
                            solar_date: peak.format("%Y-%m-%d").to_string(),
                            lunar: lunar_dt,
                            categories: rule.categories.clone(),
                            milestones: RetailCampaignMilestone {
                                prep_start: prep.format("%Y-%m-%d").to_string(),
                                early_bird: early.format("%Y-%m-%d").to_string(),
                                peak: peak.format("%Y-%m-%d").to_string(),
                            },
                            campaign_focus_vi: rule.campaign_focus_vi.clone(),
                            campaign_focus_en: rule.campaign_focus_en.clone(),
                        });
                    }
                }
            }

            // 2. Algorithmic monthly Mung 1 & Ram observances
            let monthly_cats = vec![
                RetailCategory::Religious,
                RetailCategory::FoodBeverage,
                RetailCategory::Shopping,
            ];
            let matches_cat = category.is_none_or(|c| monthly_cats.contains(&c));
            if matches_cat {
                for month in 1..=12 {
                    let mut variants = vec![false];
                    if has_leap
                        && self
                            .calendar
                            .lunar_month_days(month, lunar_year, true, tz_hours)
                            .is_ok()
                    {
                        variants.push(true);
                    }
                    for is_leap in variants {
                        for day in [1, 15] {
                            if let Some((peak, lunar_dt)) = self.resolve_single_lunar_occurrence(
                                day,
                                month,
                                lunar_year,
                                is_leap,
                                Day30Policy::Skip,
                                tz_hours,
                            )? {
                                if peak < start_solar || peak >= end_exclusive {
                                    continue;
                                }
                                let (name_vi, name_en) = if day == 1 {
                                    (
                                        format!(
                                            "Mùng 1 Tháng {month}{}",
                                            if is_leap { " Nhuận" } else { "" }
                                        ),
                                        format!(
                                            "Lunar New Moon (1/{month}{})",
                                            if is_leap { " Leap" } else { "" }
                                        ),
                                    )
                                } else {
                                    (
                                        format!(
                                            "Ngày Rằm Tháng {month}{}",
                                            if is_leap { " Nhuận" } else { "" }
                                        ),
                                        format!(
                                            "Lunar Full Moon (15/{month}{})",
                                            if is_leap { " Leap" } else { "" }
                                        ),
                                    )
                                };
                                let prep = peak - Duration::days(5);
                                let early = peak - Duration::days(2);
                                results.push(RetailEventOccurrence {
                                    id: "monthly_mung_1_ram".to_string(),
                                    name_vi,
                                    name_en,
                                    solar_date: peak.format("%Y-%m-%d").to_string(),
                                    lunar: lunar_dt,
                                    categories: monthly_cats.clone(),
                                    milestones: RetailCampaignMilestone {
                                        prep_start: prep.format("%Y-%m-%d").to_string(),
                                        early_bird: early.format("%Y-%m-%d").to_string(),
                                        peak: peak.format("%Y-%m-%d").to_string(),
                                    },
                                    campaign_focus_vi: "Trái cây tươi, hoa cúng, thực phẩm chay và mâm lễ Mùng 1 / Rằm hằng tháng.".to_string(),
                                    campaign_focus_en: "Fresh fruit, incense flowers, plant-based meals, and monthly 1st/15th essentials.".to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }

        results.sort_by(|a, b| {
            a.solar_date
                .cmp(&b.solar_date)
                .then_with(|| a.id.cmp(&b.id))
        });
        Ok(results)
    }
}

impl RetailCalendarService for RetailCalendarEngine {
    fn retail_events_for_year(
        &self,
        year: i32,
        category: Option<RetailCategory>,
        time_zone: f64,
    ) -> Result<RetailYearResponse, CalendarError> {
        validate_year_bounds(year)?;
        let (_, tz_hours) = validate_timezone(time_zone)?;
        let start =
            NaiveDate::from_ymd_opt(year, 1, 1).ok_or(CalendarError::ConversionOutOfRange)?;
        let end_exclusive =
            NaiveDate::from_ymd_opt(year + 1, 1, 1).ok_or(CalendarError::ConversionOutOfRange)?;
        let events = self.collect_for_window(start, end_exclusive, category, tz_hours)?;
        Ok(RetailYearResponse {
            year,
            category_filter: category,
            count: events.len(),
            events,
        })
    }

    fn upcoming_retail_peaks(
        &self,
        from_date: NaiveDate,
        days: i32,
        category: Option<RetailCategory>,
        time_zone: f64,
    ) -> Result<RetailUpcomingResponse, CalendarError> {
        if !(1..=MAX_RETAIL_HORIZON_DAYS).contains(&days) {
            return Err(CalendarError::InvalidSolarDate(format!(
                "days horizon {days} must be within 1..={MAX_RETAIL_HORIZON_DAYS}"
            )));
        }
        validate_year_bounds(from_date.year())?;
        let (_, tz_hours) = validate_timezone(time_zone)?;
        let end_exclusive = from_date + Duration::days(i64::from(days));
        let events = self.collect_for_window(from_date, end_exclusive, category, tz_hours)?;
        Ok(RetailUpcomingResponse {
            from_date: from_date.format("%Y-%m-%d").to_string(),
            days,
            category_filter: category,
            count: events.len(),
            events,
        })
    }

    fn replace_catalog(&self, catalog: Arc<RetailCatalog>) {
        self.catalog.store(catalog);
    }
}

/// Cordis plugin providing [`RetailCalendarService`].
pub struct RetailCalendarPlugin;

impl Plugin for RetailCalendarPlugin {
    fn meta(&self) -> PluginMeta {
        PluginMeta::new("retail_vn")
            .requires::<dyn CalendarService>()
            .provides::<dyn RetailCalendarService>()
    }

    fn mount<'a>(&'a self, ctx: Context, config: Value) -> BoxFuture<'a, Result<(), KernelError>> {
        Box::pin(async move {
            let cfg: RetailConfig = serde_json::from_value(config)
                .map_err(|e| KernelError::ConfigError(format!("retail_vn config: {e}")))?;
            let path = resolve_catalog_path(&cfg);
            let catalog = load_retail_catalog(&path).await?;
            let calendar = ctx.inject::<dyn CalendarService>()?;
            let service: Arc<dyn RetailCalendarService> =
                Arc::new(RetailCalendarEngine::new(calendar, catalog));
            ctx.provide::<dyn RetailCalendarService>(service);
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::sync::Arc;

    use chrono::NaiveDate;
    use serde_json::json;

    use crate::contracts::calendar::CalendarService;
    use crate::contracts::retail::{RetailCalendarService, RetailCategory};
    use crate::kernel::reconcile::PluginFactoryRegistry;
    use crate::kernel::Kernel;
    use crate::plugins::calendar_vn::VietnamCalendarEngine;
    use crate::plugins::calendar_vn::VietnamCalendarPlugin;
    use crate::plugins::config_reloader::{reload_from_disk, RuntimeConfigAnchor};
    use crate::plugins::holidays_vn::VietnamHolidaysPlugin;
    use crate::plugins::retail_vn::loader::parse_retail_yaml;
    use crate::plugins::retail_vn::{RetailCalendarEngine, RetailCalendarPlugin};

    fn make_engine_from_disk() -> RetailCalendarEngine {
        let cal: Arc<dyn CalendarService> = Arc::new(VietnamCalendarEngine::new(256));
        let yaml = std::fs::read_to_string(Path::new("config/retail_events.yml")).unwrap();
        let cat = parse_retail_yaml(&yaml, "config/retail_events.yml").unwrap();
        RetailCalendarEngine::new(cal, cat)
    }

    #[test]
    fn test_retail_events_2025_peaks_milestones_and_filters() {
        let engine = make_engine_from_disk();
        let year_2025 = engine.retail_events_for_year(2025, None, 7.0).unwrap();

        let find_event = |id: &str| {
            year_2025
                .events
                .iter()
                .find(|e| e.id == id)
                .expect("missing retail event")
        };

        let tat_nien = find_event("tat_nien_giao_thua");
        assert_eq!(tat_nien.solar_date, "2025-01-28");
        assert_eq!(tat_nien.milestones.peak, "2025-01-28");
        assert_eq!(tat_nien.milestones.early_bird, "2025-01-14");
        assert_eq!(tat_nien.milestones.prep_start, "2024-12-29");

        assert_eq!(find_event("ngay_via_than_tai").solar_date, "2025-02-07");
        assert_eq!(find_event("trung_thu").solar_date, "2025-10-06");
        assert!(
            year_2025
                .events
                .iter()
                .filter(|e| e.id == "monthly_mung_1_ram")
                .count()
                >= 24
        );

        let shopping = engine
            .retail_events_for_year(2025, Some(RetailCategory::Shopping), 7.0)
            .unwrap();
        assert!(shopping
            .events
            .iter()
            .all(|e| e.categories.contains(&RetailCategory::Shopping)));
    }

    #[test]
    fn test_retail_upcoming_and_strict_yaml_validation() {
        let engine = make_engine_from_disk();
        let up = engine
            .upcoming_retail_peaks(NaiveDate::from_ymd_opt(2025, 1, 20).unwrap(), 30, None, 7.0)
            .unwrap();
        assert!(up.events.iter().any(|e| e.id == "tet_nguyen_dan"));

        assert!(engine
            .upcoming_retail_peaks(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(), 0, None, 7.0)
            .is_err());
        assert!(engine
            .upcoming_retail_peaks(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(), 367, None, 7.0)
            .is_err());

        let bad_yamls = [
            "version: 2\nevents: []\n",
            "version: 1\nevents:\n  - id: bad\n    unknown_field: true\n",
            "version: 1\nevents:\n  - id: dup\n    name_vi: \"A\"\n    name_en: \"A\"\n    lunar_month: 1\n    lunar_day: 1\n    categories: [festival]\n    prep_lead_days: 5\n    early_bird_lead_days: 2\n    campaign_focus_vi: \"F\"\n    campaign_focus_en: \"F\"\n  - id: dup\n    name_vi: \"B\"\n    name_en: \"B\"\n    lunar_month: 2\n    lunar_day: 2\n    categories: [festival]\n    prep_lead_days: 5\n    early_bird_lead_days: 2\n    campaign_focus_vi: \"F\"\n    campaign_focus_en: \"F\"\n",
            "version: 1\nevents:\n  - id: bad_lead\n    name_vi: \"A\"\n    name_en: \"A\"\n    lunar_month: 1\n    lunar_day: 1\n    categories: [festival]\n    prep_lead_days: 2\n    early_bird_lead_days: 10\n    campaign_focus_vi: \"F\"\n    campaign_focus_en: \"F\"\n",
            "version: 1\nevents:\n  - id: \"\"\n    name_vi: \"A\"\n    name_en: \"A\"\n    lunar_month: 1\n    lunar_day: 1\n    categories: [festival]\n    prep_lead_days: 5\n    early_bird_lead_days: 2\n    campaign_focus_vi: \"F\"\n    campaign_focus_en: \"F\"\n",
            "version: 1\nevents:\n  - id: \" leading_space\"\n    name_vi: \"A\"\n    name_en: \"A\"\n    lunar_month: 1\n    lunar_day: 1\n    categories: [festival]\n    prep_lead_days: 5\n    early_bird_lead_days: 2\n    campaign_focus_vi: \"F\"\n    campaign_focus_en: \"F\"\n",
            "version: 1\nevents:\n  - id: empty_vi\n    name_vi: \"\"\n    name_en: \"A\"\n    lunar_month: 1\n    lunar_day: 1\n    categories: [festival]\n    prep_lead_days: 5\n    early_bird_lead_days: 2\n    campaign_focus_vi: \"F\"\n    campaign_focus_en: \"F\"\n",
            "version: 1\nevents:\n  - id: empty_en\n    name_vi: \"A\"\n    name_en: \"\"\n    lunar_month: 1\n    lunar_day: 1\n    categories: [festival]\n    prep_lead_days: 5\n    early_bird_lead_days: 2\n    campaign_focus_vi: \"F\"\n    campaign_focus_en: \"F\"\n",
            "version: 1\nevents:\n  - id: empty_fvi\n    name_vi: \"A\"\n    name_en: \"A\"\n    lunar_month: 1\n    lunar_day: 1\n    categories: [festival]\n    prep_lead_days: 5\n    early_bird_lead_days: 2\n    campaign_focus_vi: \"\"\n    campaign_focus_en: \"F\"\n",
            "version: 1\nevents:\n  - id: empty_fen\n    name_vi: \"A\"\n    name_en: \"A\"\n    lunar_month: 1\n    lunar_day: 1\n    categories: [festival]\n    prep_lead_days: 5\n    early_bird_lead_days: 2\n    campaign_focus_vi: \"F\"\n    campaign_focus_en: \"\"\n",
            "version: 1\nevents:\n  - id: neg_prep\n    name_vi: \"A\"\n    name_en: \"A\"\n    lunar_month: 1\n    lunar_day: 1\n    categories: [festival]\n    prep_lead_days: -1\n    early_bird_lead_days: 0\n    campaign_focus_vi: \"F\"\n    campaign_focus_en: \"F\"\n",
            "version: 1\nevents:\n  - id: neg_early\n    name_vi: \"A\"\n    name_en: \"A\"\n    lunar_month: 1\n    lunar_day: 1\n    categories: [festival]\n    prep_lead_days: 5\n    early_bird_lead_days: -1\n    campaign_focus_vi: \"F\"\n    campaign_focus_en: \"F\"\n",
        ];
        for bad in &bad_yamls {
            assert!(
                parse_retail_yaml(bad, "test.yml").is_err(),
                "expected failure for:\n{bad}"
            );
        }

        // Valid zero-lead boundary (prep == early_bird == 0) must succeed
        let zero_lead = "version: 1\nevents:\n  - id: same_day\n    name_vi: \"A\"\n    name_en: \"A\"\n    lunar_month: 1\n    lunar_day: 1\n    categories: [festival]\n    prep_lead_days: 0\n    early_bird_lead_days: 0\n    campaign_focus_vi: \"F\"\n    campaign_focus_en: \"F\"\n";
        assert!(parse_retail_yaml(zero_lead, "zero.yml").is_ok());
    }

    #[tokio::test]
    async fn test_retail_fail_safe_hot_reload_via_config_reloader() {
        let dir = tempfile::tempdir().unwrap();
        let hol_dir = dir.path().join("holidays");
        std::fs::create_dir_all(&hol_dir).unwrap();
        std::fs::copy("config/holidays/2025.yml", hol_dir.join("2025.yml")).unwrap();
        std::fs::copy(
            "config/retail_events.yml",
            dir.path().join("retail_events.yml"),
        )
        .unwrap();

        let kernel = Kernel::new();
        kernel
            .mount("calendar_vn", Arc::new(VietnamCalendarPlugin), json!({}))
            .await
            .unwrap();
        kernel
            .mount(
                "holidays_vn",
                Arc::new(VietnamHolidaysPlugin),
                json!({ "holidays_dir": hol_dir.to_string_lossy() }),
            )
            .await
            .unwrap();
        kernel
            .mount(
                "retail_vn",
                Arc::new(RetailCalendarPlugin),
                json!({ "config_base_dir": dir.path().to_string_lossy() }),
            )
            .await
            .unwrap();

        let mut reg = PluginFactoryRegistry::new();
        reg.register("calendar_vn", || Arc::new(VietnamCalendarPlugin));
        reg.register("holidays_vn", || Arc::new(VietnamHolidaysPlugin));
        reg.register("retail_vn", || Arc::new(RetailCalendarPlugin));

        let anchor = RuntimeConfigAnchor {
            kernel: kernel.clone(),
            factories: Arc::new(reg),
            config_dir: dir.path().to_path_buf(),
            profile: "serve".to_string(),
            port_override: None,
            bind_override: Some(false),
        };

        // Write invalid YAML -> reload fails cleanly and preserves existing events
        std::fs::write(
            dir.path().join("retail_events.yml"),
            "version: 99\nevents: []\n",
        )
        .unwrap();
        assert!(reload_from_disk(&anchor, "holidays").await.is_err());
        let svc = kernel.inject::<dyn RetailCalendarService>().unwrap();
        let yr = svc.retail_events_for_year(2025, None, 7.0).unwrap();
        assert!(yr.events.iter().any(|e| e.id == "ngay_via_than_tai"));

        // Write valid custom retail YAML -> reload swaps atomically
        let custom_yaml = r#"version: 1
events:
  - id: custom_spring_sale
    name_vi: "Lễ Hội Mùa Xuân Custom"
    name_en: "Custom Spring Promo"
    lunar_month: 2
    lunar_day: 2
    categories: [shopping]
    prep_lead_days: 7
    early_bird_lead_days: 3
    campaign_focus_vi: "Khuyến mãi tháng 2 Âm lịch."
    campaign_focus_en: "Second lunar month promo."
"#;
        std::fs::write(dir.path().join("retail_events.yml"), custom_yaml).unwrap();
        reload_from_disk(&anchor, "holidays").await.unwrap();
        let yr2 = svc.retail_events_for_year(2025, None, 7.0).unwrap();
        assert!(yr2.events.iter().any(|e| e.id == "custom_spring_sale"));
        assert!(!yr2.events.iter().any(|e| e.id == "ngay_via_than_tai"));

        kernel.shutdown().await;
    }
}
