//! `HttpServerPlugin`: wait-free `ArcSwap<Router>` & `ArcSwap<OpenApi>` server host.

use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context as TaskContext, Poll};

use arc_swap::ArcSwap;
use axum::body::Body;
use axum::extract::{FromRequestParts, Query};
use axum::http::request::Parts;
use axum::http::{Request, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::{Json, Router};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::Value;
use tower::Service;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::{error, info};
use utoipa::openapi::{InfoBuilder, OpenApi, OpenApiBuilder};

use crate::contracts::http::{HttpRouterService, RouteDisposer};
use crate::contracts::models::{CalendarError, ErrorResponse};
use crate::kernel::context::Context;
use crate::kernel::effect::BoxFuture;
use crate::kernel::error::KernelError;
use crate::kernel::fiber::{Plugin, PluginMeta};

/// Centralized HTTP API error mapped to JSON `{ "error": "..." }` payloads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppError {
    /// Client validation or malformed query error (`HTTP 400`).
    BadRequest(String),
    /// Unknown resource or missing session (`HTTP 404`).
    NotFound(String),
    /// Backpressure or transient capacity exhaustion (`HTTP 503`).
    ServiceUnavailable(String),
    /// Unexpected internal error (`HTTP 500`).
    Internal(String),
}

impl From<CalendarError> for AppError {
    fn from(err: CalendarError) -> Self {
        Self::BadRequest(err.to_string())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            Self::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            Self::ServiceUnavailable(msg) => (StatusCode::SERVICE_UNAVAILABLE, msg),
            Self::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };
        (status, Json(ErrorResponse { error: message })).into_response()
    }
}

/// Custom Axum query extractor converting deserialization rejections into JSON `AppError::BadRequest`.
#[derive(Debug, Clone, Copy)]
pub struct JsonQuery<T>(pub T);

impl<S, T> FromRequestParts<S> for JsonQuery<T>
where
    S: Send + Sync,
    T: DeserializeOwned,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        Query::<T>::from_request_parts(parts, state)
            .await
            .map(|Query(val)| Self(val))
            .map_err(|err| AppError::BadRequest(err.body_text()))
    }
}

#[derive(Clone)]
struct RouteFragment {
    id: u64,
    router: Router,
    openapi: Option<OpenApi>,
}

#[derive(Clone)]
struct OpenApiInfo {
    id: u64,
    title: String,
    version: String,
    description: String,
}

struct RouterEngineInner {
    fragments: Mutex<Vec<RouteFragment>>,
    infos: Mutex<Vec<OpenApiInfo>>,
    composed_router: ArcSwap<Router>,
    composed_openapi: ArcSwap<OpenApi>,
    next_id: AtomicU64,
}

/// Hot-swappable `HttpRouterService` backed by `ArcSwap<Router>` and `ArcSwap<OpenApi>`.
#[derive(Clone)]
pub struct HttpRouterEngine {
    inner: Arc<RouterEngineInner>,
}

impl HttpRouterEngine {
    /// Creates an empty router engine with default permissive CORS and HTTP tracing layers.
    pub fn new() -> Self {
        let engine = Self {
            inner: Arc::new(RouterEngineInner {
                fragments: Mutex::new(Vec::new()),
                infos: Mutex::new(Vec::new()),
                composed_router: ArcSwap::from_pointee(Router::new()),
                composed_openapi: ArcSwap::from_pointee(default_openapi()),
                next_id: AtomicU64::new(1),
            }),
        };
        engine.rebuild();
        engine
    }

    fn rebuild(&self) {
        let fragments = {
            let guard = self
                .inner
                .fragments
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            guard.clone()
        };
        let active_info = {
            let guard = self.inner.infos.lock().unwrap_or_else(|e| e.into_inner());
            guard.last().cloned()
        };

        let cors = CorsLayer::new()
            .allow_methods(Any)
            .allow_origin(Any)
            .allow_headers(Any);

        let mut router = Router::new();
        for frag in &fragments {
            router = router.merge(frag.router.clone());
        }
        router = router.layer(cors).layer(TraceLayer::new_for_http());

        let mut openapi = match active_info {
            Some(info) => OpenApiBuilder::new()
                .info(
                    InfoBuilder::new()
                        .title(info.title)
                        .version(info.version)
                        .description(Some(info.description))
                        .build(),
                )
                .build(),
            None => default_openapi(),
        };

        for frag in &fragments {
            if let Some(sub) = &frag.openapi {
                openapi.merge(sub.clone());
            }
        }

        self.inner.composed_router.store(Arc::new(router));
        self.inner.composed_openapi.store(Arc::new(openapi));
    }
}

impl Default for HttpRouterEngine {
    fn default() -> Self {
        Self::new()
    }
}

fn default_openapi() -> OpenApi {
    OpenApiBuilder::new()
        .info(
            InfoBuilder::new()
                .title("Vietnam Lunar Calendar API")
                .version(env!("CARGO_PKG_VERSION"))
                .description(Some("API for Lunar and Solar dates."))
                .build(),
        )
        .build()
}

impl HttpRouterService for HttpRouterEngine {
    fn mount_routes(&self, router: Router, openapi: Option<OpenApi>) -> RouteDisposer {
        let id = self.inner.next_id.fetch_add(1, Ordering::Relaxed);
        {
            let mut guard = self
                .inner
                .fragments
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            guard.push(RouteFragment {
                id,
                router,
                openapi,
            });
        }
        self.rebuild();

        let engine = self.clone();
        Box::new(move || {
            {
                let mut guard = engine
                    .inner
                    .fragments
                    .lock()
                    .unwrap_or_else(|e| e.into_inner());
                guard.retain(|frag| frag.id != id);
            }
            engine.rebuild();
        })
    }

    fn set_openapi_info(
        &self,
        title: String,
        version: String,
        description: String,
    ) -> RouteDisposer {
        let id = self.inner.next_id.fetch_add(1, Ordering::Relaxed);
        {
            let mut guard = self.inner.infos.lock().unwrap_or_else(|e| e.into_inner());
            guard.push(OpenApiInfo {
                id,
                title,
                version,
                description,
            });
        }
        self.rebuild();

        let engine = self.clone();
        Box::new(move || {
            {
                let mut guard = engine.inner.infos.lock().unwrap_or_else(|e| e.into_inner());
                guard.retain(|item| item.id != id);
            }
            engine.rebuild();
        })
    }

    fn current_router(&self) -> Router {
        (**self.inner.composed_router.load()).clone()
    }

    fn current_openapi(&self) -> OpenApi {
        (**self.inner.composed_openapi.load()).clone()
    }
}

/// Wait-free `tower::Service` dispatching every request to the latest `ArcSwap<Router>` snapshot.
#[derive(Clone)]
struct DynamicRouterService {
    engine: HttpRouterEngine,
}

impl Service<Request<Body>> for DynamicRouterService {
    type Response = Response;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut TaskContext<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let mut router = self.engine.current_router();
        Box::pin(async move { router.call(req).await })
    }
}

#[derive(Debug, Clone, Deserialize)]
struct HttpServerConfig {
    #[serde(default = "default_host")]
    host: String,
    #[serde(default)]
    port: Option<u16>,
    #[serde(default = "default_bind")]
    bind: bool,
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_bind() -> bool {
    true
}

impl Default for HttpServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: None,
            bind: true,
        }
    }
}

/// Resolves HTTP port with precedence: YAML/CLI `cfg_port` -> `env_port` -> `env_http_port` -> `8080`.
pub fn resolve_port_from(
    cfg_port: Option<u16>,
    env_port: Option<&str>,
    env_http_port: Option<&str>,
) -> u16 {
    if let Some(p) = cfg_port {
        return p;
    }
    for raw in [env_port, env_http_port].into_iter().flatten() {
        if let Ok(parsed) = raw.parse::<u16>() {
            return parsed;
        }
    }
    8080
}

/// Resolves HTTP port with precedence: config/CLI override -> `PORT` -> `HTTP_PORT` -> `8080`.
fn resolve_port(cfg_port: Option<u16>) -> u16 {
    let env_port = std::env::var("PORT").ok();
    let env_http_port = std::env::var("HTTP_PORT").ok();
    resolve_port_from(cfg_port, env_port.as_deref(), env_http_port.as_deref())
}

/// Cordis plugin providing `dyn HttpRouterService` and hosting the Axum TCP listener.
#[derive(Debug, Default)]
pub struct HttpServerPlugin;

impl Plugin for HttpServerPlugin {
    fn meta(&self) -> PluginMeta {
        PluginMeta::new("http_server").provides::<dyn HttpRouterService>()
    }

    fn mount<'a>(&'a self, ctx: Context, config: Value) -> BoxFuture<'a, Result<(), KernelError>> {
        Box::pin(async move {
            let cfg: HttpServerConfig = serde_json::from_value(config)
                .map_err(|e| KernelError::ConfigError(format!("http_server config: {e}")))?;

            let engine = HttpRouterEngine::new();
            let svc: Arc<dyn HttpRouterService> = Arc::new(engine.clone());
            ctx.provide::<dyn HttpRouterService>(svc);

            if cfg.bind {
                let port = resolve_port(cfg.port);
                let addr = format!("{}:{port}", cfg.host);
                let listener = tokio::net::TcpListener::bind(&addr).await.map_err(|e| {
                    KernelError::MountFailed {
                        fiber_id: ctx.fiber_id().to_string(),
                        reason: format!("failed to bind {addr}: {e}"),
                    }
                })?;

                let bound_addr = listener.local_addr().map(|a| a.to_string()).unwrap_or(addr);
                info!(addr = %bound_addr, "HTTP server listening");
                info!("Swagger UI at http://{}/swagger-ui", bound_addr);
                info!("Remote MCP SSE endpoint at http://{}/mcp/sse", bound_addr);

                let app = Router::new().fallback_service(DynamicRouterService { engine });

                ctx.spawn("http_server_accept", move |token| async move {
                    let serve_res = axum::serve(listener, app)
                        .with_graceful_shutdown(async move {
                            token.cancelled().await;
                        })
                        .await;
                    if let Err(err) = serve_res {
                        error!(error = %err, "Axum HTTP server terminated with error");
                    }
                });
            }

            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::routing::get;
    use http_body_util::BodyExt;
    use serde_json::json;
    use tower::ServiceExt;

    #[derive(Deserialize)]
    struct SampleQuery {
        num: i32,
    }

    #[tokio::test]
    async fn test_app_error_and_json_query_rejection() {
        let res = AppError::BadRequest("bad".into()).into_response();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);

        let nf = AppError::NotFound("missing".into()).into_response();
        assert_eq!(nf.status(), StatusCode::NOT_FOUND);

        let su = AppError::ServiceUnavailable("full".into()).into_response();
        assert_eq!(su.status(), StatusCode::SERVICE_UNAVAILABLE);

        let internal = AppError::Internal("boom".into()).into_response();
        assert_eq!(internal.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let int_bytes = internal.into_body().collect().await.unwrap().to_bytes();
        let int_json: ErrorResponse = serde_json::from_slice(&int_bytes).unwrap();
        assert_eq!(int_json.error, "boom");

        let app = Router::new().route(
            "/check",
            get(|JsonQuery(q): JsonQuery<SampleQuery>| async move { Json(q.num) }),
        );
        let ok_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/check?num=42")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(ok_resp.status(), StatusCode::OK);

        let bad_resp = app
            .oneshot(
                Request::builder()
                    .uri("/check?num=abc")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(bad_resp.status(), StatusCode::BAD_REQUEST);
        let bad_bytes = bad_resp.into_body().collect().await.unwrap().to_bytes();
        let err_payload: ErrorResponse = serde_json::from_slice(&bad_bytes).unwrap();
        assert!(!err_payload.error.is_empty());
    }

    #[tokio::test]
    async fn test_hot_swappable_router_mount_and_dispose() {
        let engine = HttpRouterEngine::new();
        let disposer =
            engine.mount_routes(Router::new().route("/ping", get(|| async { "pong" })), None);

        let resp = engine
            .current_router()
            .oneshot(Request::builder().uri("/ping").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        disposer();
        let resp_after = engine
            .current_router()
            .oneshot(Request::builder().uri("/ping").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp_after.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_bind_failure_returns_kernel_error_without_panic() {
        // Bind a listener first, then try mounting HttpServerPlugin on the exact same port.
        let occupied = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = occupied.local_addr().unwrap().port();

        let kernel = crate::kernel::Kernel::new();
        let err = kernel
            .mount(
                "http_server",
                Arc::new(HttpServerPlugin),
                json!({ "host": "127.0.0.1", "port": port, "bind": true }),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, KernelError::MountFailed { .. }));
    }

    #[test]
    fn test_resolve_port_precedence() {
        assert_eq!(
            resolve_port_from(Some(9090), Some("8081"), Some("8082")),
            9090
        );
        assert_eq!(resolve_port_from(None, Some("8081"), Some("8082")), 8081);
        assert_eq!(resolve_port_from(None, None, Some("8082")), 8082);
        assert_eq!(resolve_port_from(None, Some("bad"), Some("8082")), 8082);
        assert_eq!(resolve_port_from(None, Some("bad"), Some("99999")), 8080);
        assert_eq!(resolve_port_from(None, None, None), 8080);
    }
}
