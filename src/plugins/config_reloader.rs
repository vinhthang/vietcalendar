//! `ConfigReloaderPlugin`: non-blocking debounced `notify` + Unix `SIGHUP` live YAML reloader.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::contracts::holidays::HolidayService;
use crate::contracts::retail::RetailCalendarService;
use crate::kernel::context::Context;
use crate::kernel::effect::BoxFuture;
use crate::kernel::error::KernelError;
use crate::kernel::fiber::{Plugin, PluginMeta};
use crate::kernel::reconcile::PluginFactoryRegistry;
use crate::kernel::Kernel;
use crate::plugins::holidays_vn::loader::load_holidays_dir;
use crate::plugins::retail_vn::loader::load_retail_catalog;
use crate::profiles::load_profile_entries;

/// Runtime metadata shared with `ConfigReloaderPlugin` so live reconciles target the active profile.
#[derive(Debug, Clone)]
pub struct RuntimeConfigAnchor {
    /// Running kernel handle.
    pub kernel: Kernel,
    /// Registered plugin factories for reconciliation.
    pub factories: Arc<PluginFactoryRegistry>,
    /// Configuration directory containing `cordis.yml`, `cordis.patch.yml`, and `holidays/`.
    pub config_dir: PathBuf,
    /// Active profile name inside `cordis.yml`.
    pub profile: String,
    /// Optional port override supplied by CLI / environment.
    pub port_override: Option<u16>,
    /// Optional HTTP bind override (used in headless tests).
    pub bind_override: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
struct ReloaderConfig {
    #[serde(default = "default_debounce_ms")]
    debounce_ms: u64,
    #[serde(default = "default_holidays_subdir")]
    holidays_dir: String,
}

fn default_debounce_ms() -> u64 {
    200
}

fn default_holidays_subdir() -> String {
    "holidays".to_string()
}

impl Default for ReloaderConfig {
    fn default() -> Self {
        Self {
            debounce_ms: default_debounce_ms(),
            holidays_dir: default_holidays_subdir(),
        }
    }
}

/// Applies a single fail-safe YAML reload cycle for holiday tables and Cordis patches.
///
/// If any YAML file is malformed or fails validation, the error is returned and no active
/// schedules or kernel fibers are modified.
///
/// # Errors
/// Returns `KernelError::ConfigError` or reconciliation error if parsing/validation fails.
pub async fn reload_from_disk(
    anchor: &RuntimeConfigAnchor,
    holidays_subdir: &str,
) -> Result<(), KernelError> {
    // 1. Stage and validate per-year holiday YAML files before mutating active schedules.
    let holidays_path = if Path::new(holidays_subdir).is_absolute() {
        PathBuf::from(holidays_subdir)
    } else {
        anchor.config_dir.join(holidays_subdir)
    };
    let new_schedules = load_holidays_dir(&holidays_path).await?;

    // 1b. Stage and validate retail_events.yml if present in config_dir.
    let retail_path = anchor.config_dir.join("retail_events.yml");
    let new_retail_catalog = if tokio::fs::try_exists(&retail_path).await.unwrap_or(false) {
        Some(load_retail_catalog(&retail_path).await?)
    } else {
        None
    };

    // 2. Stage and validate cordis.yml / cordis.patch.yml if present.
    let cordis_path = anchor.config_dir.join("cordis.yml");
    let patch_path = anchor.config_dir.join("cordis.patch.yml");
    let has_cordis = tokio::fs::try_exists(&cordis_path).await.unwrap_or(false);
    let has_patch = tokio::fs::try_exists(&patch_path).await.unwrap_or(false);

    let resolved_opt = if has_cordis || has_patch {
        Some(
            load_profile_entries(
                &anchor.config_dir,
                &anchor.profile,
                anchor.port_override,
                anchor.bind_override,
            )
            .await?,
        )
    } else {
        None
    };

    // 3. Reconcile kernel BEFORE committing new holiday schedules so failure leaves state intact.
    if let Some(entries) = resolved_opt {
        anchor.kernel.reconcile(&anchor.factories, &entries).await?;
    }
    if let Some(hol_svc) = anchor.kernel.inject::<dyn HolidayService>() {
        hol_svc.replace_schedules(new_schedules);
    }
    if let Some(catalog) = new_retail_catalog {
        if let Some(retail_svc) = anchor.kernel.inject::<dyn RetailCalendarService>() {
            retail_svc.replace_catalog(catalog);
        }
    }

    anchor
        .kernel
        .emit_reload(anchor.config_dir.display().to_string());
    Ok(())
}

/// Cordis plugin watching `config/` via non-blocking `notify` + `SIGHUP` for live hot-reload.
#[derive(Debug, Default)]
pub struct ConfigReloaderPlugin;

impl Plugin for ConfigReloaderPlugin {
    fn meta(&self) -> PluginMeta {
        PluginMeta::new("config_reloader")
    }

    fn mount<'a>(&'a self, ctx: Context, config: Value) -> BoxFuture<'a, Result<(), KernelError>> {
        Box::pin(async move {
            let cfg: ReloaderConfig = serde_json::from_value(config)
                .map_err(|e| KernelError::ConfigError(format!("config_reloader config: {e}")))?;

            let Some(anchor) = ctx.try_inject::<RuntimeConfigAnchor>() else {
                // Mounted standalone without RuntimeConfigAnchor (e.g. unit test without disk anchor).
                return Ok(());
            };

            if !tokio::fs::try_exists(&anchor.config_dir)
                .await
                .unwrap_or(false)
            {
                info!(
                    dir = %anchor.config_dir.display(),
                    "config directory missing; config_reloader parked"
                );
                return Ok(());
            }

            let (tx, mut rx) = mpsc::channel::<()>(16);
            let tx_watch = tx.clone();
            let mut watcher: RecommendedWatcher = notify::recommended_watcher(move |res| {
                if let Ok(_event) = res {
                    let _ = tx_watch.try_send(());
                }
            })
            .map_err(|e| KernelError::MountFailed {
                fiber_id: ctx.fiber_id().to_string(),
                reason: format!("failed to create OS file watcher: {e}"),
            })?;

            watcher
                .watch(&anchor.config_dir, RecursiveMode::Recursive)
                .map_err(|e| KernelError::MountFailed {
                    fiber_id: ctx.fiber_id().to_string(),
                    reason: format!("failed watching {}: {e}", anchor.config_dir.display()),
                })?;

            // Keep OS watcher alive until LIFO disposal of this fiber.
            let watcher_box = Arc::new(std::sync::Mutex::new(Some(watcher)));
            let watcher_drop = Arc::clone(&watcher_box);
            ctx.on_dispose_sync(move || {
                let mut guard = watcher_drop.lock().unwrap_or_else(|e| e.into_inner());
                drop(guard.take());
            });

            let debounce = Duration::from_millis(cfg.debounce_ms.max(20));
            let holidays_subdir = cfg.holidays_dir.clone();
            ctx.spawn("config_reloader_loop", move |token| async move {
                run_reload_loop(anchor, holidays_subdir, debounce, &mut rx, token).await;
            });

            Ok(())
        })
    }
}

async fn run_reload_loop(
    anchor: Arc<RuntimeConfigAnchor>,
    holidays_subdir: String,
    debounce: Duration,
    rx: &mut mpsc::Receiver<()>,
    token: tokio_util::sync::CancellationToken,
) {
    #[cfg(unix)]
    let mut sighup = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::hangup()).ok();

    loop {
        let triggered = tokio::select! {
            biased;
            _ = token.cancelled() => break,
            msg = rx.recv() => {
                if msg.is_none() {
                    break;
                }
                true
            }
            _ = wait_sighup(&mut sighup) => true,
        };

        if !triggered {
            continue;
        }

        // Debounce bursty filesystem events cleanly while honoring cancellation.
        tokio::select! {
            biased;
            _ = token.cancelled() => break,
            _ = tokio::time::sleep(debounce) => {}
        }
        while rx.try_recv().is_ok() {}

        // Spawn outside config_reloader's EffectScope so self-reconfigure/disable cannot deadlock.
        let reload_anchor = Arc::clone(&anchor);
        let reload_dir = holidays_subdir.clone();
        let reload_handle =
            tokio::spawn(async move { reload_from_disk(&reload_anchor, &reload_dir).await });
        let outcome = tokio::select! {
            biased;
            _ = token.cancelled() => break,
            res = reload_handle => res,
        };
        match outcome {
            Ok(Ok(())) => info!("hot-reloaded configuration and holiday schedules"),
            Ok(Err(err)) => {
                warn!(error = %err, "hot-reload validation failed; retaining last known-good state");
            }
            Err(join_err) => {
                warn!(error = %join_err, "hot-reload worker terminated abnormally");
            }
        }
    }
}

#[cfg(unix)]
async fn wait_sighup(sig: &mut Option<tokio::signal::unix::Signal>) {
    match sig.as_mut() {
        Some(s) => {
            s.recv().await;
        }
        None => std::future::pending::<()>().await,
    }
}

#[cfg(not(unix))]
async fn wait_sighup(_sig: &mut Option<()>) {
    std::future::pending::<()>().await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::event::KernelEvent;
    use crate::kernel::fiber::FiberState;
    use crate::profiles::{boot_profile, build_default_factories};
    use chrono::NaiveDate;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_hot_reload_patching_config_reloader_itself_does_not_deadlock() {
        let temp = TempDir::new().unwrap();
        let holidays_dir = temp.path().join("holidays");
        fs::create_dir_all(&holidays_dir).unwrap();
        fs::copy("config/cordis.yml", temp.path().join("cordis.yml")).unwrap();
        fs::copy(
            "config/cordis.patch.yml",
            temp.path().join("cordis.patch.yml"),
        )
        .unwrap();
        fs::copy(
            "config/retail_events.yml",
            temp.path().join("retail_events.yml"),
        )
        .unwrap();
        fs::copy("config/holidays/2026.yml", holidays_dir.join("2026.yml")).unwrap();

        let kernel = boot_profile(temp.path(), "serve", None, Some(false))
            .await
            .unwrap();
        let mut events = kernel.subscribe();

        // 1. Patch config_reloader's own config -> causes reconcile to dispose & remount config_reloader!
        fs::write(
            temp.path().join("cordis.patch.yml"),
            "patches:\n  - id: config_reloader\n    config:\n      debounce_ms: 50\n",
        )
        .unwrap();

        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if let Ok(KernelEvent::ConfigReloaded { .. }) = events.recv().await {
                    break;
                }
            }
        })
        .await
        .expect("timed out waiting for self-reconfigure reload without deadlock");
        assert_eq!(
            kernel.fiber_state("config_reloader"),
            Some(FiberState::Active)
        );

        // 2. Patch config_reloader to enabled: false -> reload loop disposes itself cleanly!
        fs::write(
            temp.path().join("cordis.patch.yml"),
            "patches:\n  - id: config_reloader\n    enabled: false\n",
        )
        .unwrap();

        tokio::time::timeout(Duration::from_secs(5), async {
            while kernel.fiber_state("config_reloader").is_some() {
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("timed out waiting for self-disable of config_reloader without deadlock");

        tokio::time::timeout(Duration::from_secs(2), kernel.shutdown())
            .await
            .expect("kernel.shutdown deadlocked after reloader self-disable");
    }

    #[tokio::test]
    async fn test_reload_from_disk_does_not_mutate_holidays_when_reconcile_fails() {
        let temp = TempDir::new().unwrap();
        let holidays_dir = temp.path().join("holidays");
        fs::create_dir_all(&holidays_dir).unwrap();
        fs::copy("config/cordis.yml", temp.path().join("cordis.yml")).unwrap();
        fs::copy(
            "config/cordis.patch.yml",
            temp.path().join("cordis.patch.yml"),
        )
        .unwrap();
        fs::copy(
            "config/retail_events.yml",
            temp.path().join("retail_events.yml"),
        )
        .unwrap();
        fs::copy("config/holidays/2026.yml", holidays_dir.join("2026.yml")).unwrap();

        let kernel = boot_profile(temp.path(), "serve", None, Some(false))
            .await
            .unwrap();
        let factories = Arc::new(build_default_factories());
        let anchor = RuntimeConfigAnchor {
            kernel: kernel.clone(),
            factories,
            config_dir: temp.path().to_path_buf(),
            profile: "serve".to_string(),
            port_override: None,
            bind_override: Some(false),
        };
        let hol = kernel.inject::<dyn HolidayService>().unwrap();

        fs::write(
            holidays_dir.join("2032.yml"),
            "year: 2032\nsource: \"test\"\nholidays:\n  - date: \"2032-05-03\"\n    name: \"Test Holiday Mon\"\n    category: FixedSolar\nworking_weekends: []\n",
        )
        .unwrap();
        fs::write(
            temp.path().join("cordis.patch.yml"),
            "patches:\n  - id: bad_fiber\n    plugin: nonexistent_plugin\n    enabled: true\n",
        )
        .unwrap();

        let err = reload_from_disk(&anchor, "holidays").await.unwrap_err();
        assert!(matches!(err, KernelError::UnknownPlugin(_)));
        assert!(!hol.has_schedule(2032));
        assert!(!hol.is_holiday(NaiveDate::from_ymd_opt(2032, 5, 3).unwrap()));

        kernel.shutdown().await;
    }

    #[tokio::test]
    async fn test_reload_from_disk_retains_schedule_on_holiday_data_validation_error() {
        let temp = TempDir::new().unwrap();
        let holidays_dir = temp.path().join("holidays");
        fs::create_dir_all(&holidays_dir).unwrap();
        fs::copy("config/cordis.yml", temp.path().join("cordis.yml")).unwrap();
        fs::copy(
            "config/cordis.patch.yml",
            temp.path().join("cordis.patch.yml"),
        )
        .unwrap();
        fs::copy(
            "config/retail_events.yml",
            temp.path().join("retail_events.yml"),
        )
        .unwrap();
        fs::copy("config/holidays/2026.yml", holidays_dir.join("2026.yml")).unwrap();

        let kernel = boot_profile(temp.path(), "serve", None, Some(false))
            .await
            .unwrap();
        let factories = Arc::new(build_default_factories());
        let anchor = RuntimeConfigAnchor {
            kernel: kernel.clone(),
            factories,
            config_dir: temp.path().to_path_buf(),
            profile: "serve".to_string(),
            port_override: None,
            bind_override: Some(false),
        };
        let hol = kernel.inject::<dyn HolidayService>().unwrap();
        assert_eq!(hol.year_holidays(2026).entries().len(), 13);

        fs::write(
            holidays_dir.join("2026.yml"),
            r#"
year: 2026
source: "conflicting-2026"
holidays:
  - date: "2026-08-22"
    name: "Conflict Sat"
    category: Compensatory
working_weekends:
  - "2026-08-22"
"#,
        )
        .unwrap();

        let err = reload_from_disk(&anchor, "holidays").await.unwrap_err();
        assert!(matches!(err, KernelError::ConfigError(_)));
        assert_eq!(hol.year_holidays(2026).entries().len(), 13);
        assert!(!hol.is_holiday(NaiveDate::from_ymd_opt(2026, 8, 22).unwrap()));

        kernel.shutdown().await;
    }
}
