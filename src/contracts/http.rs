//! Service contract (`HttpRouterService`) for hot-swappable Axum routes and OpenAPI fragments.

use axum::Router;
use utoipa::openapi::OpenApi;

/// Disposer handle removing a mounted route fragment and OpenAPI spec entries on unwind.
pub type RouteDisposer = Box<dyn FnOnce() + Send>;

/// Pluggable HTTP routing & OpenAPI composition capability provided by `HttpServerPlugin`.
pub trait HttpRouterService: Send + Sync + 'static {
    /// Mounts an Axum `router` fragment and optional `OpenApi` spec into the live server snapshot.
    ///
    /// Dropping or invoking the returned disposer removes only this fragment and rebuilds
    /// the live `ArcSwap<Router>` and `ArcSwap<OpenApi>` atomically.
    fn mount_routes(&self, router: Router, openapi: Option<OpenApi>) -> RouteDisposer;

    /// Updates the top-level OpenAPI `info` metadata (`title`, `version`, `description`).
    fn set_openapi_info(
        &self,
        title: String,
        version: String,
        description: String,
    ) -> RouteDisposer;

    /// Returns a cheap wait-free snapshot of the current composed Axum `Router`.
    fn current_router(&self) -> Router;

    /// Returns a wait-free snapshot of the current composed OpenAPI 3.0 document.
    fn current_openapi(&self) -> OpenApi;
}
