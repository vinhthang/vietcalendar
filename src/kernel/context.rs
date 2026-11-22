//! Scoped execution context (`Context`) passed to Cordis plugins during `mount`.

use std::future::Future;
use std::sync::Arc;

use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::kernel::effect::EffectScope;
use crate::kernel::error::KernelError;
use crate::kernel::event::{EventBus, KernelEvent};
use crate::kernel::fiber::Plugin;
use crate::kernel::{fork_child_inner, KernelInner};

/// Scoped Cordis execution context bound to a single fiber's lifetime.
#[derive(Clone)]
pub struct Context {
    pub(crate) fiber_id: String,
    pub(crate) scope: EffectScope,
    pub(crate) inner: Arc<KernelInner>,
}

impl Context {
    /// Returns the identifier of the owning fiber.
    pub fn fiber_id(&self) -> &str {
        &self.fiber_id
    }

    /// Returns a child `CancellationToken` that cancels when this fiber disposes.
    pub fn token(&self) -> CancellationToken {
        self.scope.token()
    }

    /// Returns a reference to the kernel lifecycle event bus.
    pub fn events(&self) -> &EventBus {
        &self.inner.events
    }

    /// Registers a typed service `Arc<T>` into the wait-free `ServiceRegistry` and
    /// attaches an automatic LIFO removal disposer to this fiber's `EffectScope`.
    pub fn provide<T>(&self, service: Arc<T>)
    where
        T: ?Sized + Send + Sync + 'static,
    {
        let (type_id, type_name) = self
            .inner
            .registry
            .insert::<T>(self.fiber_id.clone(), service);
        self.inner.events.emit(KernelEvent::ServiceProvided {
            type_name,
            fiber_id: self.fiber_id.clone(),
        });

        let inner = Arc::clone(&self.inner);
        let owner = self.fiber_id.clone();
        self.scope.on_dispose_sync(move || {
            if let Some(removed_name) = inner.registry.remove_owned(type_id, &owner) {
                inner.events.emit(KernelEvent::ServiceRemoved {
                    type_name: removed_name,
                    fiber_id: owner,
                });
            }
        });
    }

    /// Performs a wait-free lookup of a required service `T`.
    ///
    /// # Errors
    /// Returns `KernelError::ServiceNotRegistered` if `T` is not currently active.
    pub fn inject<T>(&self) -> Result<Arc<T>, KernelError>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        self.inner
            .registry
            .get::<T>()
            .ok_or_else(|| KernelError::ServiceNotRegistered(std::any::type_name::<T>()))
    }

    /// Performs a wait-free optional lookup of a service `T`.
    pub fn try_inject<T>(&self) -> Option<Arc<T>>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        self.inner.registry.get::<T>()
    }

    /// Registers an asynchronous effect disposer executed in LIFO order on disposal.
    pub fn on_dispose<F, Fut>(&self, disposer: F)
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.scope.on_dispose(disposer);
    }

    /// Registers a synchronous effect disposer executed in LIFO order on disposal.
    pub fn on_dispose_sync<F>(&self, disposer: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.scope.on_dispose_sync(disposer);
    }

    /// Registers a reversible effect: executes `setup` immediately and schedules `teardown` in LIFO order.
    pub async fn effect<S, Fut, D, DFut>(&self, setup: S, teardown: D)
    where
        S: FnOnce() -> Fut,
        Fut: Future<Output = ()>,
        D: FnOnce() -> DFut + Send + 'static,
        DFut: Future<Output = ()> + Send + 'static,
    {
        setup().await;
        self.scope.on_dispose(teardown);
    }

    /// Spawns a background task bound to this fiber's `CancellationToken`.
    pub fn spawn<F, Fut>(&self, name: impl Into<String>, task_fn: F)
    where
        F: FnOnce(CancellationToken) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.scope.spawn(name, task_fn);
    }

    /// Forks a child plugin fiber under this fiber's supervision tree.
    ///
    /// Child fibers automatically dispose before their parent during teardown.
    ///
    /// # Errors
    /// Returns `KernelError` on duplicate ID, dependency cycle, or mount failure.
    pub async fn fork<P>(
        &self,
        child_id: impl Into<String>,
        plugin: Arc<P>,
        config: Value,
    ) -> Result<(), KernelError>
    where
        P: Plugin + 'static,
    {
        let plugin_dyn: Arc<dyn Plugin> = plugin;
        fork_child_inner(
            &self.inner,
            child_id.into(),
            plugin_dyn,
            config,
            &self.fiber_id,
        )
        .await
    }
}
