//! Broadcast event bus for Cordis kernel lifecycle transitions.

use tokio::sync::broadcast;

/// Events emitted when fibers mount/dispose, services appear/disappear, or configs reload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KernelEvent {
    /// A fiber entered the `Waiting` state pending required services.
    FiberWaiting {
        /// Unique fiber identifier.
        id: String,
        /// Plugin factory/type name.
        plugin: &'static str,
    },
    /// A fiber completed `mount` and became `Active`.
    FiberMounted {
        /// Unique fiber identifier.
        id: String,
        /// Plugin factory/type name.
        plugin: &'static str,
    },
    /// A fiber finished LIFO disposal.
    FiberDisposed {
        /// Unique fiber identifier.
        id: String,
    },
    /// A typed service became available in the `ServiceRegistry`.
    ServiceProvided {
        /// Concrete Rust type name of the service.
        type_name: &'static str,
        /// Identifier of the providing fiber.
        fiber_id: String,
    },
    /// A typed service was retracted from the `ServiceRegistry`.
    ServiceRemoved {
        /// Concrete Rust type name of the service.
        type_name: &'static str,
        /// Identifier of the retracting fiber.
        fiber_id: String,
    },
    /// A live configuration reload completed.
    ConfigReloaded {
        /// Description or path of the reloaded configuration source.
        source: String,
    },
}

/// Non-blocking broadcast bus for kernel lifecycle telemetry and hooks.
#[derive(Debug, Clone)]
pub struct EventBus {
    tx: broadcast::Sender<KernelEvent>,
}

impl EventBus {
    /// Creates an event bus with the given bounded ring capacity.
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity.max(16));
        Self { tx }
    }

    /// Broadcasts an event to active subscribers without blocking when zero listeners exist.
    pub fn emit(&self, event: KernelEvent) {
        let _ = self.tx.send(event);
    }

    /// Subscribes to subsequent kernel lifecycle events.
    pub fn subscribe(&self) -> broadcast::Receiver<KernelEvent> {
        self.tx.subscribe()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(128)
    }
}
