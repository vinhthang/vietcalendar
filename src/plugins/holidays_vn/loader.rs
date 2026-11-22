//! Non-blocking async YAML loader for per-year Vietnam holiday schedules (`config/holidays/<year>.yml`).

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{Datelike, NaiveDate, Weekday};
use serde::Deserialize;
use tracing::debug;

use crate::contracts::models::{HolidayCategory, HolidayEntry, YearHolidaySchedule};
use crate::kernel::error::KernelError;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawHolidayItem {
    date: String,
    name: String,
    category: HolidayCategory,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawYearHolidayFile {
    year: i32,
    #[serde(default)]
    source: String,
    #[serde(default)]
    holidays: Vec<RawHolidayItem>,
    #[serde(default)]
    working_weekends: Vec<String>,
}

/// Validates semantic integrity of a parsed per-year Vietnam holiday schedule.
///
/// Enforces:
/// - `year` within `1..=9999`
/// - Non-empty holiday names, matching `year`, and no duplicate dates within `holidays`
/// - `working_weekends` dates match `year`, fall on `Sat`/`Sun`, are unique, and do not
///   conflict with `holidays`
///
/// # Errors
/// Returns `KernelError::ConfigError` on any domain or consistency violation.
pub fn validate_holiday_data(
    year: i32,
    entries: &[HolidayEntry],
    working_weekends: &[NaiveDate],
    source_label: &str,
) -> Result<(), KernelError> {
    if !(1..=9999).contains(&year) {
        return Err(KernelError::ConfigError(format!(
            "{source_label}: schedule year {year} out of supported range 1..=9999"
        )));
    }

    let mut holiday_dates = HashSet::with_capacity(entries.len());
    for entry in entries {
        if entry.name.trim().is_empty() {
            return Err(KernelError::ConfigError(format!(
                "{source_label}: holiday entry on '{}' has empty name",
                entry.date
            )));
        }
        if entry.date.year() != year {
            return Err(KernelError::ConfigError(format!(
                "{source_label}: holiday date '{}' does not match schedule year {year}",
                entry.date
            )));
        }
        if !holiday_dates.insert(entry.date) {
            return Err(KernelError::ConfigError(format!(
                "{source_label}: duplicate holiday date '{}'",
                entry.date
            )));
        }
    }

    let mut seen_weekends = HashSet::with_capacity(working_weekends.len());
    for &ws in working_weekends {
        if ws.year() != year {
            return Err(KernelError::ConfigError(format!(
                "{source_label}: working_weekends date '{ws}' does not match schedule year {year}"
            )));
        }
        if !matches!(ws.weekday(), Weekday::Sat | Weekday::Sun) {
            return Err(KernelError::ConfigError(format!(
                "{source_label}: working_weekends date '{ws}' is a {}, not a weekend (Sat/Sun)",
                ws.weekday()
            )));
        }
        if !seen_weekends.insert(ws) {
            return Err(KernelError::ConfigError(format!(
                "{source_label}: duplicate working_weekends date '{ws}'"
            )));
        }
        if holiday_dates.contains(&ws) {
            return Err(KernelError::ConfigError(format!(
                "{source_label}: date '{ws}' cannot appear in both holidays and working_weekends"
            )));
        }
    }

    Ok(())
}

/// Parses and validates a single YAML holiday schedule string into an immutable `YearHolidaySchedule`.
///
/// # Errors
/// Returns `KernelError::ConfigError` if YAML syntax, unknown fields, `YYYY-MM-DD` dates, or
/// domain integrity invariants fail.
pub fn parse_year_schedule_yaml(
    yaml_content: &str,
    source_label: &str,
) -> Result<YearHolidaySchedule, KernelError> {
    let raw: RawYearHolidayFile = serde_yaml_ng::from_str(yaml_content)
        .map_err(|e| KernelError::ConfigError(format!("{source_label}: invalid YAML: {e}")))?;

    let mut entries = Vec::with_capacity(raw.holidays.len());
    for item in raw.holidays {
        let date = NaiveDate::parse_from_str(&item.date, "%Y-%m-%d").map_err(|e| {
            KernelError::ConfigError(format!(
                "{source_label}: invalid holiday date '{}': {e}",
                item.date
            ))
        })?;
        entries.push(HolidayEntry {
            date,
            name: item.name,
            category: item.category,
        });
    }

    let mut working_weekends = Vec::with_capacity(raw.working_weekends.len());
    for ws in raw.working_weekends {
        let date = NaiveDate::parse_from_str(&ws, "%Y-%m-%d").map_err(|e| {
            KernelError::ConfigError(format!(
                "{source_label}: invalid working_weekends date '{ws}': {e}"
            ))
        })?;
        working_weekends.push(date);
    }

    validate_holiday_data(raw.year, &entries, &working_weekends, source_label)?;

    let source = if raw.source.trim().is_empty() {
        source_label.to_string()
    } else {
        raw.source
    };

    Ok(YearHolidaySchedule::new(
        raw.year,
        source,
        entries,
        working_weekends,
    ))
}

/// Loads all `*.yml` / `*.yaml` holiday schedules under `dir` using async `tokio::fs`.
///
/// Missing directories return an empty map cleanly (allowing weekends-only fallback or plugin
/// overrides). Any invalid YAML file or duplicate year declaration aborts loading with
/// `KernelError::ConfigError` so existing schedules are never partially replaced.
///
/// # Errors
/// Returns `KernelError::ConfigError` if any `.yml`/`.yaml` file fails parsing/validation or if
/// multiple files declare the same `year`.
pub async fn load_holidays_dir(
    dir: &Path,
) -> Result<HashMap<i32, Arc<YearHolidaySchedule>>, KernelError> {
    if !tokio::fs::try_exists(dir).await.unwrap_or(false) {
        debug!(path = %dir.display(), "holidays directory absent; defaulting to weekends-only fallback");
        return Ok(HashMap::new());
    }

    let mut read_dir = tokio::fs::read_dir(dir)
        .await
        .map_err(|e| KernelError::ConfigError(format!("failed reading {}: {e}", dir.display())))?;

    let mut yaml_paths: Vec<PathBuf> = Vec::new();
    while let Some(entry) = read_dir
        .next_entry()
        .await
        .map_err(|e| KernelError::ConfigError(format!("failed iterating {}: {e}", dir.display())))?
    {
        let path = entry.path();
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        if ext == "yml" || ext == "yaml" {
            yaml_paths.push(path);
        }
    }
    yaml_paths.sort();

    let mut loaded = HashMap::with_capacity(yaml_paths.len());
    let mut seen_sources: HashMap<i32, String> = HashMap::with_capacity(yaml_paths.len());
    for path in yaml_paths {
        let content = tokio::fs::read_to_string(&path).await.map_err(|e| {
            KernelError::ConfigError(format!("failed reading {}: {e}", path.display()))
        })?;
        let label = path.display().to_string();
        let schedule = parse_year_schedule_yaml(&content, &label)?;
        if let Some(prev) = seen_sources.insert(schedule.year, label.clone()) {
            return Err(KernelError::ConfigError(format!(
                "duplicate holiday schedule for year {} ({prev} and {label})",
                schedule.year
            )));
        }
        loaded.insert(schedule.year, Arc::new(schedule));
    }

    Ok(loaded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn assert_config_err(res: Result<YearHolidaySchedule, KernelError>, needle: &str) {
        match res {
            Err(KernelError::ConfigError(msg)) => assert!(
                msg.contains(needle),
                "expected error containing '{needle}', got: {msg}"
            ),
            other => panic!("expected Err(ConfigError('{needle}')), got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_load_holidays_dir_accepts_official_2024_2025_2026() {
        let loaded = load_holidays_dir(Path::new("config/holidays"))
            .await
            .unwrap();
        assert!(loaded.contains_key(&2024));
        assert!(loaded.contains_key(&2025));
        assert!(loaded.contains_key(&2026));
    }

    #[test]
    fn test_rejects_holiday_date_year_mismatch() {
        let yaml = r#"
year: 2026
holidays:
  - date: "2025-05-01"
    name: "Wrong Year Holiday"
    category: FixedSolar
"#;
        assert_config_err(
            parse_year_schedule_yaml(yaml, "2026.yml"),
            "does not match schedule year",
        );
    }

    #[test]
    fn test_rejects_duplicate_holiday_date() {
        let yaml = r#"
year: 2026
holidays:
  - date: "2026-01-01"
    name: "New Year A"
    category: FixedSolar
  - date: "2026-01-01"
    name: "New Year B"
    category: FixedSolar
"#;
        assert_config_err(
            parse_year_schedule_yaml(yaml, "2026.yml"),
            "duplicate holiday date",
        );
    }

    #[test]
    fn test_rejects_working_weekend_year_mismatch() {
        let yaml = r#"
year: 2026
holidays: []
working_weekends:
  - "2025-04-26"
"#;
        assert_config_err(
            parse_year_schedule_yaml(yaml, "2026.yml"),
            "does not match schedule year",
        );
    }

    #[test]
    fn test_rejects_non_weekend_in_working_weekends() {
        let yaml = r#"
year: 2026
holidays: []
working_weekends:
  - "2026-08-24"
"#;
        assert_config_err(parse_year_schedule_yaml(yaml, "2026.yml"), "not a weekend");
    }

    #[test]
    fn test_rejects_duplicate_working_weekend_date() {
        let yaml = r#"
year: 2026
holidays: []
working_weekends:
  - "2026-08-22"
  - "2026-08-22"
"#;
        assert_config_err(
            parse_year_schedule_yaml(yaml, "2026.yml"),
            "duplicate working_weekends date",
        );
    }

    #[test]
    fn test_rejects_holiday_and_working_weekend_conflict() {
        let yaml = r#"
year: 2026
holidays:
  - date: "2026-08-22"
    name: "Conflicting Entry"
    category: Compensatory
working_weekends:
  - "2026-08-22"
"#;
        assert_config_err(
            parse_year_schedule_yaml(yaml, "2026.yml"),
            "both holidays and working_weekends",
        );
    }

    #[test]
    fn test_rejects_blank_holiday_name_invalid_year_and_unknown_field() {
        let blank_name = r#"
year: 2026
holidays:
  - date: "2026-01-01"
    name: "   "
    category: FixedSolar
"#;
        assert!(matches!(
            parse_year_schedule_yaml(blank_name, "2026.yml"),
            Err(KernelError::ConfigError(_))
        ));

        for bad_year in [0, 10000] {
            let yml = format!("year: {bad_year}\nholidays: []\n");
            assert!(
                matches!(
                    parse_year_schedule_yaml(&yml, "bad.yml"),
                    Err(KernelError::ConfigError(_))
                ),
                "expected ConfigError for year {bad_year}"
            );
        }

        let unknown_key = r#"
year: 2026
holidays: []
working_weekend:
  - "2026-08-22"
"#;
        assert!(matches!(
            parse_year_schedule_yaml(unknown_key, "typo.yml"),
            Err(KernelError::ConfigError(_))
        ));
    }

    #[tokio::test]
    async fn test_load_holidays_dir_rejects_duplicate_year_files() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("2026_a.yml"), "year: 2026\nholidays: []\n").unwrap();
        fs::write(
            temp.path().join("2026_b.yaml"),
            "year: 2026\nholidays: []\n",
        )
        .unwrap();

        match load_holidays_dir(temp.path()).await {
            Err(KernelError::ConfigError(msg)) => {
                assert!(
                    msg.contains("duplicate holiday schedule for year 2026"),
                    "unexpected error message: {msg}"
                );
            }
            other => panic!("expected duplicate year ConfigError, got: {other:?}"),
        }
    }
}
