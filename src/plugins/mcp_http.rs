//! `McpHttpPlugin`: lock-free `ArcSwap` SSE session registry with bounded `try_send` backpressure.

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context as TaskContext, Poll};

use arc_swap::ArcSwap;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TrySendError;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::Stream;
use tokio_stream::StreamExt as _;

use crate::contracts::http::HttpRouterService;
use crate::contracts::mcp::{JsonRpcRequest, JsonRpcResponse, McpRegistryService};
use crate::contracts::models::ErrorResponse;
use crate::kernel::context::Context;
use crate::kernel::effect::BoxFuture;
use crate::kernel::error::KernelError;
use crate::kernel::fiber::{Plugin, PluginMeta};
use crate::plugins::http_server::JsonQuery;

/// Outcome of attempting to push an SSE event to a registered session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendOutcome {
    /// Event queued into the session's bounded channel.
    Delivered,
    /// Unknown or already-closed `sessionId`.
    NotFound,
    /// Session channel buffer is full (`503 Service Unavailable` backpressure).
    Backpressure,
}

/// Lock-free read-side session map storing bounded `mpsc::Sender<Event>` channels.
#[derive(Clone)]
pub struct McpSessionManager {
    sessions: Arc<ArcSwap<HashMap<String, mpsc::Sender<Event>>>>,
    write_lock: Arc<Mutex<()>>,
    channel_capacity: usize,
}

impl McpSessionManager {
    /// Creates a new `McpSessionManager` with per-session bounded `channel_capacity`.
    pub fn new(channel_capacity: usize) -> Self {
        Self {
            sessions: Arc::new(ArcSwap::from_pointee(HashMap::new())),
            write_lock: Arc::new(Mutex::new(())),
            channel_capacity: channel_capacity.max(1),
        }
    }

    /// Registers a bounded sender channel for `session_id`.
    pub fn register(&self, session_id: String, tx: mpsc::Sender<Event>) {
        let _guard = self.write_lock.lock().unwrap_or_else(|e| e.into_inner());
        let mut next = (**self.sessions.load()).clone();
        next.insert(session_id, tx);
        self.sessions.store(Arc::new(next));
    }

    /// Removes `session_id` synchronously without spawning background tasks.
    pub fn remove(&self, session_id: &str) {
        let _guard = self.write_lock.lock().unwrap_or_else(|e| e.into_inner());
        let current = self.sessions.load();
        if current.contains_key(session_id) {
            let mut next = (**current).clone();
            next.remove(session_id);
            self.sessions.store(Arc::new(next));
        }
    }

    /// Returns `true` if `session_id` is registered and its receiver is still open.
    pub fn contains_active(&self, session_id: &str) -> bool {
        self.sessions
            .load()
            .get(session_id)
            .is_some_and(|tx| !tx.is_closed())
    }

    /// Pushes `event` non-blockingly using `mpsc::Sender::try_send`.
    pub fn try_send(&self, session_id: &str, event: Event) -> SendOutcome {
        let map = self.sessions.load();
        let Some(tx) = map.get(session_id) else {
            return SendOutcome::NotFound;
        };
        match tx.try_send(event) {
            Ok(()) => SendOutcome::Delivered,
            Err(TrySendError::Full(_)) => SendOutcome::Backpressure,
            Err(TrySendError::Closed(_)) => {
                drop(map);
                self.remove(session_id);
                SendOutcome::NotFound
            }
        }
    }

    /// Returns the number of currently tracked SSE sessions.
    pub fn active_sessions_count(&self) -> usize {
        self.sessions.load().len()
    }

    /// Returns the configured per-session channel capacity.
    pub fn channel_capacity(&self) -> usize {
        self.channel_capacity
    }
}

/// SSE response stream that unregisters its session synchronously on `Drop`.
pub struct SessionStream<S> {
    stream: S,
    session_id: String,
    session_manager: McpSessionManager,
}

impl<S: Stream<Item = Result<Event, std::convert::Infallible>> + Unpin> Stream
    for SessionStream<S>
{
    type Item = Result<Event, std::convert::Infallible>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.stream).poll_next(cx)
    }
}

impl<S> Drop for SessionStream<S> {
    fn drop(&mut self) {
        self.session_manager.remove(&self.session_id);
    }
}

#[derive(Clone)]
struct McpHttpState {
    registry: Arc<dyn McpRegistryService>,
    sessions: McpSessionManager,
}

/// Query parameters accepted by `POST /mcp/message`.
#[derive(Debug, Deserialize)]
pub struct MessageQuery {
    /// Active SSE session identifier.
    #[serde(alias = "sessionId", alias = "session_id")]
    pub session_id: String,
}

async fn sse_handler(State(state): State<McpHttpState>) -> impl IntoResponse {
    let session_id = uuid::Uuid::new_v4().to_string();
    let (tx, rx) = mpsc::channel::<Event>(state.sessions.channel_capacity());

    let endpoint_event = Event::default()
        .event("endpoint")
        .data(format!("/mcp/message?sessionId={session_id}"));
    let _ = tx.try_send(endpoint_event);

    state.sessions.register(session_id.clone(), tx);

    let stream = ReceiverStream::new(rx).map(Ok::<_, std::convert::Infallible>);
    let session_stream = SessionStream {
        stream,
        session_id,
        session_manager: state.sessions.clone(),
    };

    Sse::new(session_stream).keep_alive(KeepAlive::default())
}

async fn message_handler(
    State(state): State<McpHttpState>,
    JsonQuery(query): JsonQuery<MessageQuery>,
    Json(req): Json<JsonRpcRequest>,
) -> Response {
    if !state.sessions.contains_active(&query.session_id) {
        return (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Session not found or connection closed".to_string(),
            }),
        )
            .into_response();
    }

    let Some(resp) = state.registry.dispatch(req) else {
        return StatusCode::ACCEPTED.into_response();
    };

    let json_str = match serde_json::to_string(&resp) {
        Ok(s) => s,
        Err(err) => serde_json::to_string(&JsonRpcResponse::error(
            resp.id,
            -32603,
            format!("Serialization error: {err}"),
        ))
        .unwrap_or_else(|_| {
            r#"{"jsonrpc":"2.0","id":null,"error":{"code":-32603,"message":"internal error"}}"#
                .to_string()
        }),
    };

    let event = Event::default().event("message").data(json_str);
    match state.sessions.try_send(&query.session_id, event) {
        SendOutcome::Delivered => StatusCode::ACCEPTED.into_response(),
        SendOutcome::NotFound => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Session not found or connection closed".to_string(),
            }),
        )
            .into_response(),
        SendOutcome::Backpressure => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "SSE session send buffer full".to_string(),
            }),
        )
            .into_response(),
    }
}

async fn direct_post_handler(
    State(state): State<McpHttpState>,
    Json(req): Json<JsonRpcRequest>,
) -> Response {
    if let Some(resp) = state.registry.dispatch(req) {
        Json(resp).into_response()
    } else {
        StatusCode::NO_CONTENT.into_response()
    }
}

#[derive(Debug, Clone, Deserialize)]
struct McpHttpConfig {
    #[serde(default = "default_session_capacity")]
    session_channel_capacity: usize,
}

fn default_session_capacity() -> usize {
    64
}

impl Default for McpHttpConfig {
    fn default() -> Self {
        Self {
            session_channel_capacity: default_session_capacity(),
        }
    }
}

/// Cordis plugin mounting remote HTTP/SSE and direct POST MCP routes onto `HttpRouterService`.
#[derive(Debug, Default)]
pub struct McpHttpPlugin;

impl Plugin for McpHttpPlugin {
    fn meta(&self) -> PluginMeta {
        PluginMeta::new("mcp_http")
            .requires::<dyn McpRegistryService>()
            .requires::<dyn HttpRouterService>()
    }

    fn mount<'a>(&'a self, ctx: Context, config: Value) -> BoxFuture<'a, Result<(), KernelError>> {
        Box::pin(async move {
            let cfg: McpHttpConfig = serde_json::from_value(config)
                .map_err(|e| KernelError::ConfigError(format!("mcp_http config: {e}")))?;

            let registry = ctx.inject::<dyn McpRegistryService>()?;
            let router_svc = ctx.inject::<dyn HttpRouterService>()?;
            let sessions = McpSessionManager::new(cfg.session_channel_capacity);

            let state = McpHttpState {
                registry,
                sessions: sessions.clone(),
            };

            let router = Router::new()
                .route("/mcp/sse", get(sse_handler).post(direct_post_handler))
                .route("/mcp/message", post(message_handler))
                .route("/mcp", post(direct_post_handler))
                .with_state(state);

            let disposer = router_svc.mount_routes(router, None);
            ctx.on_dispose_sync(disposer);
            ctx.provide(Arc::new(sessions));
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_manager_bounded_try_send_and_sync_drop() {
        let manager = McpSessionManager::new(1);
        let (tx, mut rx) = mpsc::channel(1);
        manager.register("s1".into(), tx);
        assert_eq!(manager.active_sessions_count(), 1);
        assert!(manager.contains_active("s1"));

        assert_eq!(
            manager.try_send("s1", Event::default().data("first")),
            SendOutcome::Delivered
        );
        // Second send while buffer=1 is unread triggers Backpressure instead of blocking!
        assert_eq!(
            manager.try_send("s1", Event::default().data("second")),
            SendOutcome::Backpressure
        );

        let _ = rx.recv().await;
        manager.remove("s1");
        assert_eq!(manager.active_sessions_count(), 0);
        assert_eq!(
            manager.try_send("s1", Event::default().data("gone")),
            SendOutcome::NotFound
        );
    }
}
