//! Telemetry plugin configuring `tracing` log filters and gated OpenTelemetry OTLP export.

use std::sync::{Arc, Mutex};

use opentelemetry::global;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_sdk::trace::TracerProvider;
use serde::Deserialize;
use serde_json::Value;
use tracing::{info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::kernel::context::Context;
use crate::kernel::effect::BoxFuture;
use crate::kernel::error::KernelError;
use crate::kernel::fiber::{Plugin, PluginMeta};

static TRACING_INITIALIZED: Mutex<bool> = Mutex::new(false);

#[derive(Debug, Clone, Deserialize)]
struct TelemetryConfig {
    #[serde(default = "default_filter_string")]
    default_filter: String,
    #[serde(default)]
    otlp_enabled: Option<bool>,
}

fn default_filter_string() -> String {
    "info,vietcalendar_rs=debug,tower_http=debug".to_string()
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            default_filter: default_filter_string(),
            otlp_enabled: None,
        }
    }
}

fn init_global_tracing(
    default_filter: &str,
    otlp_requested: bool,
) -> (Option<TracerProvider>, Option<TracerProvider>) {
    let mut initialized = TRACING_INITIALIZED
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if *initialized {
        return (None, None);
    }

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(default_filter));

    let mut built_provider: Option<TracerProvider> = None;
    if otlp_requested {
        match opentelemetry_otlp::SpanExporter::builder()
            .with_tonic()
            .build()
        {
            Ok(exporter) => {
                let provider = TracerProvider::builder()
                    .with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)
                    .build();
                global::set_tracer_provider(provider.clone());
                built_provider = Some(provider);
            }
            Err(err) => {
                warn!(error = %err, "OTLP span exporter unavailable; using console tracing");
            }
        }
    }

    let init_res = if let Some(ref provider) = built_provider {
        let tracer = provider.tracer("vietcalendar-rs");
        let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);
        tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer())
            .with(telemetry)
            .try_init()
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer())
            .try_init()
    };

    *initialized = true;
    if init_res.is_ok() {
        info!(otlp_requested, "telemetry initialized");
        (built_provider, None)
    } else {
        (None, built_provider)
    }
}

/// Initializes structured `tracing` and optional OTLP gRPC export with LIFO provider shutdown.
#[derive(Debug, Default)]
pub struct TelemetryPlugin;

impl Plugin for TelemetryPlugin {
    fn meta(&self) -> PluginMeta {
        PluginMeta::new("telemetry")
    }

    fn mount<'a>(&'a self, ctx: Context, config: Value) -> BoxFuture<'a, Result<(), KernelError>> {
        Box::pin(async move {
            let cfg: TelemetryConfig = serde_json::from_value(config)
                .map_err(|e| KernelError::ConfigError(format!("telemetry config: {e}")))?;

            let otlp_requested = cfg.otlp_enabled.unwrap_or(false)
                || std::env::var_os("OTEL_EXPORTER_OTLP_ENDPOINT").is_some();

            let (active_provider, unused_provider) =
                init_global_tracing(&cfg.default_filter, otlp_requested);

            if let Some(unused) = unused_provider {
                let _ = tokio::task::spawn_blocking(move || unused.shutdown()).await;
            }

            let provider_slot: Arc<Mutex<Option<TracerProvider>>> =
                Arc::new(Mutex::new(active_provider));

            ctx.on_dispose(move || async move {
                let maybe_provider = provider_slot
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .take();
                if let Some(provider) = maybe_provider {
                    let _ = tokio::task::spawn_blocking(move || {
                        if let Err(err) = provider.shutdown() {
                            warn!(error = ?err, "failed to flush OpenTelemetry TracerProvider");
                        }
                    })
                    .await;
                }
            });

            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::fiber::FiberState;
    use crate::kernel::Kernel;
    use serde_json::json;

    #[tokio::test]
    async fn test_telemetry_mounts_without_otlp_collector() {
        let kernel = Kernel::new();
        kernel
            .mount(
                "telemetry",
                Arc::new(TelemetryPlugin),
                json!({ "otlp_enabled": false }),
            )
            .await
            .unwrap();
        assert_eq!(kernel.fiber_state("telemetry"), Some(FiberState::Active));
        kernel.shutdown().await;
        assert!(kernel.fiber_states().is_empty());
    }
}
