//! `VietnamHolidaysPlugin`: data-driven, lock-free Vietnam holiday service with `moka` cache.

pub mod loader;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use arc_swap::ArcSwap;
use chrono::{Datelike, NaiveDate, Weekday};
use moka::sync::Cache;
use serde::Deserialize;
use serde_json::Value;

use crate::contracts::holidays::{HolidayDisposer, HolidayService};
use crate::contracts::models::{CalendarError, YearHolidaySchedule};
use crate::kernel::context::Context;
use crate::kernel::effect::BoxFuture;
use crate::kernel::error::KernelError;
use crate::kernel::fiber::{Plugin, PluginMeta};
use crate::plugins::holidays_vn::loader::load_holidays_dir;

#[derive(Debug, Clone, Deserialize)]
struct HolidaysConfig {
    #[serde(default = "default_holidays_dir")]
    holidays_dir: String,
    #[serde(default)]
    config_base_dir: Option<String>,
    #[serde(default = "default_cache_capacity")]
    cache_capacity: u64,
}

fn default_holidays_dir() -> String {
    "holidays".to_string()
}

fn default_cache_capacity() -> u64 {
    256
}

impl Default for HolidaysConfig {
    fn default() -> Self {
        Self {
            holidays_dir: default_holidays_dir(),
            config_base_dir: None,
            cache_capacity: default_cache_capacity(),
        }
    }
}

struct HolidayMutableState {
    base_schedules: HashMap<i32, Arc<YearHolidaySchedule>>,
    plugin_overrides: HashMap<u64, Arc<YearHolidaySchedule>>,
}

/// Shared interior state wrapped by `Arc` so `register_year_schedule` disposers hold an `Arc<Inner>`
/// cleanly without violating `&self` lifetimes.
struct HolidayEngineInner {
    effective_schedules: ArcSwap<HashMap<i32, Arc<YearHolidaySchedule>>>,
    state: Mutex<HolidayMutableState>,
    next_override_id: AtomicU64,
    fallback_cache: Cache<i32, Arc<YearHolidaySchedule>>,
}

fn merge_effective_schedules(
    state: &HolidayMutableState,
) -> HashMap<i32, Arc<YearHolidaySchedule>> {
    let mut merged = state.base_schedules.clone();
    let mut best_overrides: HashMap<i32, (u64, &Arc<YearHolidaySchedule>)> = HashMap::new();
    for (&id, sched) in &state.plugin_overrides {
        best_overrides
            .entry(sched.year)
            .and_modify(|cur| {
                if id > cur.0 {
                    *cur = (id, sched);
                }
            })
            .or_insert((id, sched));
    }
    for (year, (_, sched)) in best_overrides {
        merged.insert(year, Arc::clone(sched));
    }
    merged
}

impl HolidayEngineInner {
    fn publish_locked(&self, state: &HolidayMutableState, invalidate_year: Option<i32>) {
        let merged = merge_effective_schedules(state);
        self.effective_schedules.store(Arc::new(merged));
        match invalidate_year {
            Some(year) => self.fallback_cache.invalidate(&year),
            None => self.fallback_cache.invalidate_all(),
        }
    }
}

/// Thread-safe, lock-free read-path `HolidayService` engine with reversible overlay disposers.
#[derive(Clone)]
pub struct ShareableHolidayEngine {
    inner: Arc<HolidayEngineInner>,
}

impl ShareableHolidayEngine {
    /// Creates a new data-driven holiday engine with `initial_schedules` and `cache_capacity`.
    pub fn new(
        initial_schedules: HashMap<i32, Arc<YearHolidaySchedule>>,
        cache_capacity: u64,
    ) -> Self {
        Self {
            inner: Arc::new(HolidayEngineInner {
                effective_schedules: ArcSwap::from_pointee(initial_schedules.clone()),
                state: Mutex::new(HolidayMutableState {
                    base_schedules: initial_schedules,
                    plugin_overrides: HashMap::new(),
                }),
                next_override_id: AtomicU64::new(1),
                fallback_cache: Cache::new(cache_capacity.max(16)),
            }),
        }
    }
}

impl HolidayService for ShareableHolidayEngine {
    fn is_holiday(&self, date: NaiveDate) -> bool {
        let schedules = self.inner.effective_schedules.load();
        if let Some(sched) = schedules.get(&date.year()) {
            return sched.is_holiday_date(date);
        }
        matches!(date.weekday(), Weekday::Sat | Weekday::Sun)
    }

    fn is_holiday_with_timezone(
        &self,
        date: NaiveDate,
        time_zone: f64,
    ) -> Result<bool, CalendarError> {
        if !time_zone.is_finite() || !(-12.0..=14.0).contains(&time_zone) {
            return Err(CalendarError::InvalidTimezone(time_zone));
        }
        Ok(self.is_holiday(date))
    }

    fn year_holidays(&self, year: i32) -> Arc<YearHolidaySchedule> {
        if let Some(sched) = self.inner.effective_schedules.load().get(&year) {
            return Arc::clone(sched);
        }
        self.inner
            .fallback_cache
            .get_with(year, || Arc::new(YearHolidaySchedule::weekends_only(year)))
    }

    fn has_schedule(&self, year: i32) -> bool {
        self.inner.effective_schedules.load().contains_key(&year)
    }

    fn replace_schedules(&self, schedules: HashMap<i32, Arc<YearHolidaySchedule>>) {
        let mut state = self.inner.state.lock().unwrap_or_else(|e| e.into_inner());
        state.base_schedules = schedules;
        self.inner.publish_locked(&state, None);
    }

    fn register_year_schedule(&self, schedule: YearHolidaySchedule) -> HolidayDisposer {
        let year = schedule.year;
        let id = self.inner.next_override_id.fetch_add(1, Ordering::Relaxed);
        {
            let mut state = self.inner.state.lock().unwrap_or_else(|e| e.into_inner());
            state.plugin_overrides.insert(id, Arc::new(schedule));
            self.inner.publish_locked(&state, Some(year));
        }

        let inner = Arc::clone(&self.inner);
        Box::new(move || {
            let mut state = inner.state.lock().unwrap_or_else(|e| e.into_inner());
            state.plugin_overrides.remove(&id);
            inner.publish_locked(&state, Some(year));
        })
    }
}

/// Cordis plugin loading per-year YAML holiday tables and providing `dyn HolidayService`.
#[derive(Debug, Default)]
pub struct VietnamHolidaysPlugin;

impl VietnamHolidaysPlugin {
    /// Resolves the filesystem path where per-year `holidays/*.yml` files reside.
    pub fn resolve_holidays_path(config: &Value) -> Result<PathBuf, KernelError> {
        let cfg: HolidaysConfig = serde_json::from_value(config.clone())
            .map_err(|e| KernelError::ConfigError(format!("holidays_vn config: {e}")))?;
        let dir_path = PathBuf::from(&cfg.holidays_dir);
        if dir_path.is_absolute() {
            return Ok(dir_path);
        }
        let base = cfg
            .config_base_dir
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("VIETCALENDAR_CONFIG_DIR").map(PathBuf::from))
            .unwrap_or_else(|| PathBuf::from("config"));
        Ok(base.join(dir_path))
    }
}

impl Plugin for VietnamHolidaysPlugin {
    fn meta(&self) -> PluginMeta {
        PluginMeta::new("holidays_vn").provides::<dyn HolidayService>()
    }

    fn mount<'a>(&'a self, ctx: Context, config: Value) -> BoxFuture<'a, Result<(), KernelError>> {
        Box::pin(async move {
            let cfg: HolidaysConfig = serde_json::from_value(config.clone())
                .map_err(|e| KernelError::ConfigError(format!("holidays_vn config: {e}")))?;
            let dir_path = Self::resolve_holidays_path(&config)?;
            let schedules = load_holidays_dir(&dir_path).await?;
            let engine: Arc<dyn HolidayService> =
                Arc::new(ShareableHolidayEngine::new(schedules, cfg.cache_capacity));
            ctx.provide::<dyn HolidayService>(engine);
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::models::{HolidayCategory, HolidayEntry};
    use serde_json::json;
    use std::path::Path;

    #[tokio::test]
    async fn test_official_2024_2025_2026_yaml_and_fallback() {
        let schedules = load_holidays_dir(Path::new("config/holidays"))
            .await
            .unwrap();
        let engine = ShareableHolidayEngine::new(schedules, 64);

        // 2024 official holidays + swapped working Saturday 2024-05-04 -> false
        assert!(engine.is_holiday(NaiveDate::from_ymd_opt(2024, 2, 13).unwrap()));
        assert!(engine.is_holiday(NaiveDate::from_ymd_opt(2024, 4, 29).unwrap()));
        assert!(!engine.is_holiday(NaiveDate::from_ymd_opt(2024, 5, 4).unwrap()));
        assert!(engine.is_holiday(NaiveDate::from_ymd_opt(2024, 5, 5).unwrap()));

        // 2025 official holidays + swapped working Saturday 2025-04-26 -> false
        assert!(engine.is_holiday(NaiveDate::from_ymd_opt(2025, 1, 29).unwrap()));
        assert!(engine.is_holiday(NaiveDate::from_ymd_opt(2025, 5, 2).unwrap()));
        assert!(!engine.is_holiday(NaiveDate::from_ymd_opt(2025, 4, 26).unwrap()));

        // 2026 official holidays + Hung Kings compensatory Apr 27 + swapped Sat 2026-08-22 -> false
        assert!(engine.is_holiday(NaiveDate::from_ymd_opt(2026, 4, 27).unwrap()));
        assert!(engine.is_holiday(NaiveDate::from_ymd_opt(2026, 8, 31).unwrap()));
        assert!(!engine.is_holiday(NaiveDate::from_ymd_opt(2026, 8, 22).unwrap()));
        assert_eq!(engine.year_holidays(2026).entries().len(), 13);

        // Unconfigured year 2099 -> weekends-only fallback
        assert!(!engine.has_schedule(2099));
        assert!(engine.year_holidays(2099).entries().is_empty());
        // 2099-01-01 is Thursday -> false under unconfigured weekends-only fallback
        assert!(!engine.is_holiday(NaiveDate::from_ymd_opt(2099, 1, 1).unwrap()));
        // 2099-01-03 is Saturday -> true
        assert!(engine.is_holiday(NaiveDate::from_ymd_opt(2099, 1, 3).unwrap()));
    }

    #[tokio::test]
    async fn test_reversible_plugin_schedule_override_and_cache_invalidation() {
        let engine = ShareableHolidayEngine::new(HashMap::new(), 64);
        let d = NaiveDate::from_ymd_opt(2030, 5, 1).unwrap(); // Wednesday
        assert!(!engine.is_holiday(d));

        let disposer = engine.register_year_schedule(YearHolidaySchedule::new(
            2030,
            "plugin:override",
            vec![HolidayEntry {
                date: d,
                name: "Test Labor Day".into(),
                category: HolidayCategory::FixedSolar,
            }],
            vec![],
        ));
        assert!(engine.is_holiday(d));
        assert!(engine.has_schedule(2030));

        // Disposing restores weekends-only fallback and invalidates cache.
        disposer();
        assert!(!engine.is_holiday(d));
        assert!(!engine.has_schedule(2030));
    }

    #[tokio::test]
    async fn test_holidays_plugin_mount() {
        let kernel = crate::kernel::Kernel::new();
        kernel
            .mount(
                "holidays_vn",
                Arc::new(VietnamHolidaysPlugin),
                json!({ "config_base_dir": "config", "holidays_dir": "holidays" }),
            )
            .await
            .unwrap();
        let svc = kernel.inject::<dyn HolidayService>().unwrap();
        assert!(svc.has_schedule(2026));
        kernel.shutdown().await;
    }

    #[test]
    fn test_overlapping_plugin_overrides_and_replace_schedules() {
        let engine = ShareableHolidayEngine::new(HashMap::new(), 64);
        let d1 = NaiveDate::from_ymd_opt(2030, 5, 1).unwrap(); // Wed
        let d2 = NaiveDate::from_ymd_opt(2030, 5, 2).unwrap(); // Thu
        let d3 = NaiveDate::from_ymd_opt(2030, 5, 3).unwrap(); // Fri

        let disp1 = engine.register_year_schedule(YearHolidaySchedule::new(
            2030,
            "override-1",
            vec![HolidayEntry {
                date: d1,
                name: "First Override".into(),
                category: HolidayCategory::FixedSolar,
            }],
            vec![],
        ));
        let disp2 = engine.register_year_schedule(YearHolidaySchedule::new(
            2030,
            "override-2",
            vec![HolidayEntry {
                date: d2,
                name: "Second Override".into(),
                category: HolidayCategory::Bridge,
            }],
            vec![],
        ));

        // Override #2 (higher ID) wins for 2030.
        assert!(!engine.is_holiday(d1));
        assert!(engine.is_holiday(d2));
        assert_eq!(engine.year_holidays(2030).source, "override-2");

        // Disposing #2 reveals #1.
        disp2();
        assert!(engine.is_holiday(d1));
        assert!(!engine.is_holiday(d2));
        assert_eq!(engine.year_holidays(2030).source, "override-1");

        // replace_schedules swaps underlying base while active plugin override #1 still takes precedence.
        let base_2030 = Arc::new(YearHolidaySchedule::new(
            2030,
            "base-2030",
            vec![HolidayEntry {
                date: d3,
                name: "Base Schedule".into(),
                category: HolidayCategory::Compensatory,
            }],
            vec![],
        ));
        engine.replace_schedules(HashMap::from([(2030, base_2030)]));
        assert!(engine.is_holiday(d1));
        assert!(!engine.is_holiday(d3));

        // Disposing #1 exposes base-2030.
        disp1();
        assert!(!engine.is_holiday(d1));
        assert!(engine.is_holiday(d3));
        assert_eq!(engine.year_holidays(2030).source, "base-2030");
        assert!(engine.has_schedule(2030));
    }

    #[test]
    fn test_is_holiday_with_timezone_validation() {
        let engine = ShareableHolidayEngine::new(HashMap::new(), 64);
        let mon = NaiveDate::from_ymd_opt(2030, 5, 6).unwrap();
        let sat = NaiveDate::from_ymd_opt(2030, 5, 4).unwrap();

        for bad_tz in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -12.5, 14.5] {
            assert!(matches!(
                engine.is_holiday_with_timezone(mon, bad_tz),
                Err(CalendarError::InvalidTimezone(_))
            ));
        }

        assert_eq!(engine.is_holiday_with_timezone(mon, -12.0), Ok(false));
        assert_eq!(engine.is_holiday_with_timezone(mon, 7.0), Ok(false));
        assert_eq!(engine.is_holiday_with_timezone(sat, 14.0), Ok(true));
    }
}
