//! Reversible LIFO effect scopes and cancellation-bound background task handles.

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::warn;

/// Boxed future type used across the zero-domain Cordis kernel.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

enum Disposer {
    Sync(Box<dyn FnOnce() + Send>),
    Async(Box<dyn FnOnce() -> BoxFuture<'static, ()> + Send>),
}

type SpawnedTaskList = Arc<Mutex<Vec<(String, JoinHandle<()>)>>>;

/// Tracks reversible side effects, disposers, and spawned tasks bound to a fiber's lifetime.
#[derive(Clone)]
pub struct EffectScope {
    cancel: CancellationToken,
    disposers: Arc<Mutex<Vec<Disposer>>>,
    tasks: SpawnedTaskList,
}

impl EffectScope {
    /// Creates a new `EffectScope` deriving cancellation from `parent`.
    pub fn new(parent: &CancellationToken) -> Self {
        Self {
            cancel: parent.child_token(),
            disposers: Arc::new(Mutex::new(Vec::new())),
            tasks: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Returns a child `CancellationToken` cancelled automatically on disposal.
    pub fn token(&self) -> CancellationToken {
        self.cancel.clone()
    }

    /// Registers a synchronous cleanup callback executed in LIFO order on disposal.
    pub fn on_dispose_sync<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let mut guard = self.disposers.lock().unwrap_or_else(|e| e.into_inner());
        guard.push(Disposer::Sync(Box::new(f)));
    }

    /// Registers an asynchronous cleanup callback executed in LIFO order on disposal.
    pub fn on_dispose<F, Fut>(&self, f: F)
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let mut guard = self.disposers.lock().unwrap_or_else(|e| e.into_inner());
        guard.push(Disposer::Async(Box::new(move || Box::pin(f()))));
    }

    /// Spawns a background task bound to this fiber's cancellation token.
    ///
    /// On fiber disposal or kernel shutdown, the token is cancelled and the task handle is joined.
    pub fn spawn<F, Fut>(&self, name: impl Into<String>, task_fn: F)
    where
        F: FnOnce(CancellationToken) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let task_name = name.into();
        let token = self.cancel.child_token();
        let handle = tokio::spawn(task_fn(token));
        let mut guard = self.tasks.lock().unwrap_or_else(|e| e.into_inner());
        guard.push((task_name, handle));
    }

    /// Cancels bound tasks, joins them cleanly, and unwinds registered disposers in LIFO order.
    pub async fn dispose(&self) {
        self.cancel.cancel();

        let current_task_id = tokio::task::try_id();
        let tasks = {
            let mut guard = self.tasks.lock().unwrap_or_else(|e| e.into_inner());
            std::mem::take(&mut *guard)
        };
        for (name, handle) in tasks.into_iter().rev() {
            if current_task_id.is_some_and(|cur| cur == handle.id()) {
                continue;
            }
            if let Err(err) = handle.await {
                warn!(task = %name, error = %err, "fiber background task terminated abnormally");
            }
        }

        let disposers = {
            let mut guard = self.disposers.lock().unwrap_or_else(|e| e.into_inner());
            std::mem::take(&mut *guard)
        };
        for disposer in disposers.into_iter().rev() {
            match disposer {
                Disposer::Sync(f) => f(),
                Disposer::Async(f) => f().await,
            }
        }
    }
}
