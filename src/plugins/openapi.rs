//! `OpenApiPlugin`: dynamically exposes `/api-docs/openapi.json` and Swagger UI (`/swagger-ui`).

use std::sync::Arc;

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::Value;
use utoipa_swagger_ui::{Config, SwaggerUi};

use crate::contracts::http::HttpRouterService;
use crate::kernel::context::Context;
use crate::kernel::effect::BoxFuture;
use crate::kernel::error::KernelError;
use crate::kernel::fiber::{Plugin, PluginMeta};

#[derive(Debug, Clone, Deserialize)]
struct OpenApiConfig {
    #[serde(default = "default_title")]
    title: String,
    #[serde(default = "default_version")]
    version: String,
    #[serde(default = "default_description")]
    description: String,
}

fn default_title() -> String {
    "Vietnam Lunar Calendar API".to_string()
}

fn default_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

fn default_description() -> String {
    "Non-blocking plugin-based API for Vietnamese Luni-Solar dates and public holidays.".to_string()
}

impl Default for OpenApiConfig {
    fn default() -> Self {
        Self {
            title: default_title(),
            version: default_version(),
            description: default_description(),
        }
    }
}

async fn get_openapi_json(
    State(router_svc): State<Arc<dyn HttpRouterService>>,
) -> Json<utoipa::openapi::OpenApi> {
    Json(router_svc.current_openapi())
}

/// Cordis plugin publishing dynamic composed OpenAPI JSON and Swagger UI.
#[derive(Debug, Default)]
pub struct OpenApiPlugin;

impl Plugin for OpenApiPlugin {
    fn meta(&self) -> PluginMeta {
        PluginMeta::new("openapi").requires::<dyn HttpRouterService>()
    }

    fn mount<'a>(&'a self, ctx: Context, config: Value) -> BoxFuture<'a, Result<(), KernelError>> {
        Box::pin(async move {
            let cfg: OpenApiConfig = serde_json::from_value(config)
                .map_err(|e| KernelError::ConfigError(format!("openapi config: {e}")))?;

            let router_svc = ctx.inject::<dyn HttpRouterService>()?;
            let info_disposer =
                router_svc.set_openapi_info(cfg.title, cfg.version, cfg.description);
            ctx.on_dispose_sync(info_disposer);

            // Configure Swagger UI to fetch `/api-docs/openapi.json` without claiming that route,
            // allowing our dynamic handler below to serve the live ArcSwap<OpenApi> snapshot.
            let swagger = Router::new().merge(
                SwaggerUi::new("/swagger-ui").config(Config::from("/api-docs/openapi.json")),
            );

            let dynamic_json = Router::new()
                .route("/api-docs/openapi.json", get(get_openapi_json))
                .with_state(Arc::clone(&router_svc));

            let route_disposer = router_svc.mount_routes(swagger.merge(dynamic_json), None);
            ctx.on_dispose_sync(route_disposer);
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::Kernel;
    use crate::plugins::http_server::HttpServerPlugin;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use serde_json::json;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_openapi_plugin_custom_info_and_disposal() {
        let kernel = Kernel::new();
        kernel
            .mount(
                "http_server",
                Arc::new(HttpServerPlugin),
                json!({ "bind": false }),
            )
            .await
            .unwrap();
        kernel
            .mount(
                "openapi",
                Arc::new(OpenApiPlugin),
                json!({
                    "title": "Custom API",
                    "version": "9.9.9",
                    "description": "Custom OpenAPI check"
                }),
            )
            .await
            .unwrap();

        let router_svc = kernel.inject::<dyn HttpRouterService>().unwrap();
        let resp = router_svc
            .current_router()
            .oneshot(
                Request::builder()
                    .uri("/api-docs/openapi.json")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let spec: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(spec["info"]["title"], "Custom API");
        assert_eq!(spec["info"]["version"], "9.9.9");

        // Disposing openapi removes `/api-docs/openapi.json` and restores default OpenApi metadata.
        kernel.dispose("openapi").await.unwrap();
        assert_eq!(
            router_svc.current_openapi().info.version,
            env!("CARGO_PKG_VERSION")
        );
        let resp_after = router_svc
            .current_router()
            .oneshot(
                Request::builder()
                    .uri("/api-docs/openapi.json")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp_after.status(), StatusCode::NOT_FOUND);

        kernel.shutdown().await;
    }
}
