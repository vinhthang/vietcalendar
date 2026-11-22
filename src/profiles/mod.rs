//! Profile loader, factory registry, and declarative kernel boot/reconciliation helpers.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::Value;

use crate::kernel::error::KernelError;
use crate::kernel::reconcile::{
    resolve_profile, CordisConfig, CordisPatch, PluginFactoryRegistry, ProfileEntry,
};
use crate::kernel::Kernel;
use crate::plugins::astrology_vn::AstrologyPlugin;
use crate::plugins::calendar_vn::VietnamCalendarPlugin;
use crate::plugins::canchi_solar_vn::CanChiSolarPlugin;
use crate::plugins::config_reloader::{ConfigReloaderPlugin, RuntimeConfigAnchor};
use crate::plugins::holidays_vn::VietnamHolidaysPlugin;
use crate::plugins::http_calendar::HttpCalendarPlugin;
use crate::plugins::http_discovery::HttpDiscoveryPlugin;
use crate::plugins::http_holidays::HttpHolidaysPlugin;
use crate::plugins::http_niches::{
    HttpAstrologyPlugin, HttpCanChiPlugin, HttpRecurrencePlugin, HttpRetailPlugin,
    HttpTrachNhatPlugin,
};
use crate::plugins::http_server::HttpServerPlugin;
use crate::plugins::mcp_core::McpCorePlugin;
use crate::plugins::mcp_http::McpHttpPlugin;
use crate::plugins::mcp_niches::{
    McpAstrologyPlugin, McpCanChiPlugin, McpRecurrencePlugin, McpRetailPlugin, McpTrachNhatPlugin,
};
use crate::plugins::mcp_stdio::McpStdioPlugin;
use crate::plugins::mcp_tools::McpToolsPlugin;
use crate::plugins::openapi::OpenApiPlugin;
use crate::plugins::recurrence_vn::LunarRecurrencePlugin;
use crate::plugins::retail_vn::RetailCalendarPlugin;
use crate::plugins::telemetry::TelemetryPlugin;
use crate::plugins::trach_nhat_vn::TrachNhatPlugin;

/// Embedded default `cordis.yml` used as fallback when no external config file exists on disk.
const EMBEDDED_CORDIS_YAML: &str = include_str!("../../config/cordis.yml");

/// Builds the default registry mapping YAML `plugin:` identifiers to all 28 built-in plugins.
pub fn build_default_factories() -> PluginFactoryRegistry {
    let mut reg = PluginFactoryRegistry::new();
    reg.register("telemetry", || Arc::new(TelemetryPlugin));
    reg.register("calendar_vn", || Arc::new(VietnamCalendarPlugin));
    reg.register("holidays_vn", || Arc::new(VietnamHolidaysPlugin));
    reg.register("canchi_solar_vn", || Arc::new(CanChiSolarPlugin));
    reg.register("astrology_vn", || Arc::new(AstrologyPlugin));
    reg.register("trach_nhat_vn", || Arc::new(TrachNhatPlugin));
    reg.register("recurrence_vn", || Arc::new(LunarRecurrencePlugin));
    reg.register("retail_vn", || Arc::new(RetailCalendarPlugin));
    reg.register("http_server", || Arc::new(HttpServerPlugin));
    reg.register("http_calendar", || Arc::new(HttpCalendarPlugin));
    reg.register("http_discovery", || Arc::new(HttpDiscoveryPlugin));
    reg.register("http_holidays", || Arc::new(HttpHolidaysPlugin));
    reg.register("http_canchi", || Arc::new(HttpCanChiPlugin));
    reg.register("http_astrology", || Arc::new(HttpAstrologyPlugin));
    reg.register("http_trach_nhat", || Arc::new(HttpTrachNhatPlugin));
    reg.register("http_recurrence", || Arc::new(HttpRecurrencePlugin));
    reg.register("http_retail", || Arc::new(HttpRetailPlugin));
    reg.register("openapi", || Arc::new(OpenApiPlugin));
    reg.register("mcp_core", || Arc::new(McpCorePlugin));
    reg.register("mcp_tools", || Arc::new(McpToolsPlugin));
    reg.register("mcp_canchi", || Arc::new(McpCanChiPlugin));
    reg.register("mcp_astrology", || Arc::new(McpAstrologyPlugin));
    reg.register("mcp_trach_nhat", || Arc::new(McpTrachNhatPlugin));
    reg.register("mcp_recurrence", || Arc::new(McpRecurrencePlugin));
    reg.register("mcp_retail", || Arc::new(McpRetailPlugin));
    reg.register("mcp_stdio", || Arc::new(McpStdioPlugin));
    reg.register("mcp_http", || Arc::new(McpHttpPlugin));
    reg.register("config_reloader", || Arc::new(ConfigReloaderPlugin));
    reg
}

/// Resolves the active configuration directory (`explicit` -> `VIETCALENDAR_CONFIG_DIR` -> `"config"`).
pub fn resolve_config_dir(explicit: Option<PathBuf>) -> PathBuf {
    explicit
        .or_else(|| std::env::var_os("VIETCALENDAR_CONFIG_DIR").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("config"))
}

/// Loads `cordis.yml` (falling back to embedded defaults when missing) and optional `cordis.patch.yml`,
/// injects runtime overrides (`config_base_dir`, `port`), and returns resolved profile entries.
///
/// # Errors
/// Returns `KernelError::ConfigError` or `KernelError::UnknownProfile` on invalid YAML or unknown profile.
pub async fn load_profile_entries(
    config_dir: &Path,
    profile: &str,
    port_override: Option<u16>,
    bind_http: Option<bool>,
) -> Result<Vec<ProfileEntry>, KernelError> {
    let cordis_path = config_dir.join("cordis.yml");
    let cordis_yaml = if tokio::fs::try_exists(&cordis_path).await.unwrap_or(false) {
        tokio::fs::read_to_string(&cordis_path).await.map_err(|e| {
            KernelError::ConfigError(format!("failed reading {}: {e}", cordis_path.display()))
        })?
    } else {
        EMBEDDED_CORDIS_YAML.to_string()
    };

    let cordis_cfg: CordisConfig = serde_yaml_ng::from_str(&cordis_yaml)
        .map_err(|e| KernelError::ConfigError(format!("invalid cordis.yml: {e}")))?;

    let patch_path = config_dir.join("cordis.patch.yml");
    let patch_cfg = if tokio::fs::try_exists(&patch_path).await.unwrap_or(false) {
        let patch_yaml = tokio::fs::read_to_string(&patch_path).await.map_err(|e| {
            KernelError::ConfigError(format!("failed reading {}: {e}", patch_path.display()))
        })?;
        Some(
            serde_yaml_ng::from_str::<CordisPatch>(&patch_yaml)
                .map_err(|e| KernelError::ConfigError(format!("invalid cordis.patch.yml: {e}")))?,
        )
    } else {
        None
    };

    let mut entries = resolve_profile(&cordis_cfg, patch_cfg.as_ref(), profile)?;
    let base_dir_str = config_dir.to_string_lossy().into_owned();

    for entry in &mut entries {
        if entry.plugin == "holidays_vn" || entry.plugin == "retail_vn" {
            if let Value::Object(ref mut map) = entry.config {
                map.entry("config_base_dir".to_string())
                    .or_insert_with(|| Value::String(base_dir_str.clone()));
            }
        }
        if entry.plugin == "http_server" {
            if let Value::Object(ref mut map) = entry.config {
                if let Some(port) = port_override {
                    map.insert("port".to_string(), Value::from(port));
                }
                if let Some(bind) = bind_http {
                    map.insert("bind".to_string(), Value::from(bind));
                }
            }
        }
    }

    Ok(entries)
}

/// Boots a `Kernel` instance reconciled against `config_dir` and `profile`.
///
/// Also binds a host `RuntimeConfigAnchor` so `ConfigReloaderPlugin` can
/// reconcile live file/signal changes safely.
///
/// # Errors
/// Returns `KernelError` if config parsing, dependency validation, or plugin mounting fails.
pub async fn boot_profile(
    config_dir: &Path,
    profile: &str,
    port_override: Option<u16>,
    bind_http: Option<bool>,
) -> Result<Kernel, KernelError> {
    let factories = Arc::new(build_default_factories());
    let entries = load_profile_entries(config_dir, profile, port_override, bind_http).await?;

    let kernel = Kernel::new();
    let anchor = RuntimeConfigAnchor {
        kernel: kernel.clone(),
        factories: Arc::clone(&factories),
        config_dir: config_dir.to_path_buf(),
        profile: profile.to_string(),
        port_override,
        bind_override: bind_http,
    };

    kernel.provide_host(Arc::new(anchor));
    kernel.reconcile(&factories, &entries).await?;
    Ok(kernel)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_default_serve_profile_allows_env_port_fallback() {
        let entries = load_profile_entries(Path::new("config"), "serve", None, Some(false))
            .await
            .unwrap();
        let http = entries
            .iter()
            .find(|e| e.id == "http_server")
            .expect("http_server entry");
        assert!(
            http.config.get("port").is_none(),
            "cordis.yml must not hardcode port so PORT/HTTP_PORT env vars apply"
        );

        let temp = TempDir::new().unwrap();
        fs::copy("config/cordis.yml", temp.path().join("cordis.yml")).unwrap();
        fs::write(
            temp.path().join("cordis.patch.yml"),
            "patches:\n  - id: http_server\n    config:\n      port: 9090\n",
        )
        .unwrap();

        let patched = load_profile_entries(temp.path(), "serve", None, Some(false))
            .await
            .unwrap();
        let http_patched = patched.iter().find(|e| e.id == "http_server").unwrap();
        assert_eq!(http_patched.config.get("port"), Some(&Value::from(9090)));

        let overridden = load_profile_entries(temp.path(), "serve", Some(7070), Some(false))
            .await
            .unwrap();
        let http_over = overridden.iter().find(|e| e.id == "http_server").unwrap();
        assert_eq!(http_over.config.get("port"), Some(&Value::from(7070)));
    }
}
