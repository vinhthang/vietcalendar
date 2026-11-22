//! Fiber metadata, lifecycle states, and the core `Plugin` trait.

use std::any::TypeId;
use std::sync::Arc;

use serde_json::Value;

use crate::kernel::context::Context;
use crate::kernel::effect::{BoxFuture, EffectScope};
use crate::kernel::error::KernelError;

/// Execution state of a Cordis fiber inside the kernel tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FiberState {
    /// Registered and awaiting required services before `mount` can run.
    Waiting,
    /// Currently executing `Plugin::mount`.
    Mounting,
    /// Mounted and healthy; services and effects are live.
    Active,
    /// Unwinding child fibers and LIFO effects.
    Disposing,
}

/// Declarative dependency metadata for a Cordis plugin.
#[derive(Debug, Clone)]
pub struct PluginMeta {
    /// Human-readable plugin name (also used as factory identifier).
    pub name: &'static str,
    /// Typed services (`TypeId::of::<T>()`) provided by this plugin when mounted.
    pub provides: Vec<TypeId>,
    /// Typed services (`TypeId::of::<T>()`) required before this plugin can mount.
    pub requires: Vec<TypeId>,
}

impl PluginMeta {
    /// Creates metadata with no declared `provides` or `requires`.
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            provides: Vec::new(),
            requires: Vec::new(),
        }
    }

    /// Declares that this plugin provides service `T` (supports unsized trait objects).
    pub fn provides<T: ?Sized + 'static>(mut self) -> Self {
        self.provides.push(TypeId::of::<T>());
        self
    }

    /// Declares that this plugin requires service `T` before mounting.
    pub fn requires<T: ?Sized + 'static>(mut self) -> Self {
        self.requires.push(TypeId::of::<T>());
        self
    }
}

/// Core Cordis plugin interface implemented by all domain and transport modules.
pub trait Plugin: Send + Sync {
    /// Returns dependency metadata (`provides` / `requires`) for scheduling and cycles.
    fn meta(&self) -> PluginMeta;

    /// Mounts the plugin into `ctx`, registering services, effects, and child fibers.
    fn mount<'a>(&'a self, ctx: Context, config: Value) -> BoxFuture<'a, Result<(), KernelError>>;
}

/// Internal record held by `KernelInner` per registered fiber.
pub(crate) struct FiberRecord {
    pub id: String,
    pub plugin: Arc<dyn Plugin>,
    pub meta: PluginMeta,
    pub config: Value,
    pub parent_id: Option<String>,
    pub children: Vec<String>,
    pub scope: EffectScope,
    pub state: FiberState,
}
