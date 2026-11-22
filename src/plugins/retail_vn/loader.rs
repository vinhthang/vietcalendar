//! Strict async YAML loader & validator for `config/retail_events.yml`.

use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;

use crate::contracts::retail::RetailCatalog;
use crate::kernel::error::KernelError;

/// Validates a parsed [`RetailCatalog`] against business invariants.
///
/// # Errors
/// Returns [`KernelError::ConfigError`] when version, IDs, dates, categories, or offsets fail validation.
pub(crate) fn validate_retail_catalog(catalog: &RetailCatalog) -> Result<(), KernelError> {
    if catalog.version != 1 {
        return Err(KernelError::ConfigError(format!(
            "unsupported retail catalog version {} (expected 1)",
            catalog.version
        )));
    }
    if catalog.events.is_empty() {
        return Err(KernelError::ConfigError(
            "retail catalog must contain at least one event rule".to_string(),
        ));
    }

    let mut seen_ids = HashSet::with_capacity(catalog.events.len());
    for ev in &catalog.events {
        let trimmed_id = ev.id.trim();
        if trimmed_id.is_empty()
            || trimmed_id != ev.id
            || !trimmed_id
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        {
            return Err(KernelError::ConfigError(format!(
                "invalid retail event id '{}': must be non-empty ASCII snake_case",
                ev.id
            )));
        }
        if !seen_ids.insert(trimmed_id) {
            return Err(KernelError::ConfigError(format!(
                "duplicate retail event id '{}'",
                ev.id
            )));
        }
        if ev.name_vi.trim().is_empty()
            || ev.name_en.trim().is_empty()
            || ev.campaign_focus_vi.trim().is_empty()
            || ev.campaign_focus_en.trim().is_empty()
        {
            return Err(KernelError::ConfigError(format!(
                "retail event '{}' has empty bilingual display or focus fields",
                ev.id
            )));
        }
        if !(1..=12).contains(&ev.lunar_month) {
            return Err(KernelError::ConfigError(format!(
                "retail event '{}' lunar_month {} out of 1..=12 range",
                ev.id, ev.lunar_month
            )));
        }
        if !(1..=30).contains(&ev.lunar_day) {
            return Err(KernelError::ConfigError(format!(
                "retail event '{}' lunar_day {} out of 1..=30 range",
                ev.id, ev.lunar_day
            )));
        }
        if ev.categories.is_empty() {
            return Err(KernelError::ConfigError(format!(
                "retail event '{}' must declare at least one category",
                ev.id
            )));
        }
        if ev.prep_lead_days < 0 || ev.early_bird_lead_days < 0 {
            return Err(KernelError::ConfigError(format!(
                "retail event '{}' has negative lead days",
                ev.id
            )));
        }
        if ev.prep_lead_days < ev.early_bird_lead_days {
            return Err(KernelError::ConfigError(format!(
                "retail event '{}' prep_lead_days ({}) must be >= early_bird_lead_days ({})",
                ev.id, ev.prep_lead_days, ev.early_bird_lead_days
            )));
        }
    }
    Ok(())
}

/// Parses and strictly validates a `retail_events.yml` string slice.
///
/// # Errors
/// Returns [`KernelError::ConfigError`] on malformed YAML, unknown fields, or invariant violations.
pub fn parse_retail_yaml(yaml: &str, source: &str) -> Result<Arc<RetailCatalog>, KernelError> {
    let catalog: RetailCatalog = serde_yaml_ng::from_str(yaml).map_err(|e| {
        KernelError::ConfigError(format!("invalid retail catalog YAML in {source}: {e}"))
    })?;
    validate_retail_catalog(&catalog)?;
    Ok(Arc::new(catalog))
}

/// Asynchronously reads, parses, and strictly validates `path` (`config/retail_events.yml`).
///
/// # Errors
/// Returns [`KernelError::ConfigError`] when reading or validation fails.
pub(crate) async fn load_retail_catalog(path: &Path) -> Result<Arc<RetailCatalog>, KernelError> {
    let raw = tokio::fs::read_to_string(path)
        .await
        .map_err(|e| KernelError::ConfigError(format!("failed reading {}: {e}", path.display())))?;
    parse_retail_yaml(&raw, &path.display().to_string())
}
