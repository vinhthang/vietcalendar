//! Zero-domain Cordis-style non-blocking plugin kernel.
//!
//! Provides a `Context`/`Fiber` supervision tree, wait-free `ArcSwap` service injection
//! (`provide`/`inject`), reactive dependency resolution, LIFO `EffectScope` unwinding,
//! and declarative YAML profile/patch reconciliation.

pub mod context;
pub mod effect;
pub mod error;
pub mod event;
pub mod fiber;
pub mod reconcile;
pub mod registry;

use std::any::TypeId;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use serde_json::Value;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info};

use crate::kernel::context::Context;
use crate::kernel::effect::{BoxFuture, EffectScope};
use crate::kernel::error::KernelError;
use crate::kernel::event::{EventBus, KernelEvent};
use crate::kernel::fiber::{FiberRecord, FiberState, Plugin, PluginMeta};
use crate::kernel::reconcile::{PluginFactoryRegistry, ProfileEntry};
use crate::kernel::registry::ServiceRegistry;

pub(crate) struct KernelInner {
    pub registry: ServiceRegistry,
    pub events: EventBus,
    pub root_token: CancellationToken,
    pub lifecycle_lock: tokio::sync::Mutex<()>,
    pub fibers: Mutex<HashMap<String, FiberRecord>>,
    pub mount_order: Mutex<Vec<String>>,
}

/// Cordis-style plugin kernel managing fiber lifecycles, typed services, and reversible effects.
#[derive(Clone)]
pub struct Kernel {
    inner: Arc<KernelInner>,
}

impl Kernel {
    /// Creates a fresh Cordis kernel instance with empty fiber and service tables.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(KernelInner {
                registry: ServiceRegistry::new(),
                events: EventBus::default(),
                root_token: CancellationToken::new(),
                lifecycle_lock: tokio::sync::Mutex::new(()),
                fibers: Mutex::new(HashMap::new()),
                mount_order: Mutex::new(Vec::new()),
            }),
        }
    }

    /// Returns a wait-free lookup of service `T` directly from the kernel registry.
    pub fn inject<T>(&self) -> Option<Arc<T>>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        self.inner.registry.get::<T>()
    }

    /// Registers a host-provided service outside profile fibers so `reconcile` preserves it.
    pub fn provide_host<T>(&self, service: Arc<T>)
    where
        T: ?Sized + Send + Sync + 'static,
    {
        self.inner
            .registry
            .insert::<T>("__host".to_string(), service);
    }

    /// Subscribes to kernel lifecycle events (`FiberMounted`, `FiberDisposed`, etc.).
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<KernelEvent> {
        self.inner.events.subscribe()
    }

    /// Returns the current lifecycle state of fiber `id`, if registered.
    pub fn fiber_state(&self, id: &str) -> Option<FiberState> {
        let guard = self.inner.fibers.lock().unwrap_or_else(|e| e.into_inner());
        guard.get(id).map(|f| f.state)
    }

    /// Returns a snapshot of all registered fiber IDs and their current states.
    pub fn fiber_states(&self) -> HashMap<String, FiberState> {
        let guard = self.inner.fibers.lock().unwrap_or_else(|e| e.into_inner());
        guard.iter().map(|(k, v)| (k.clone(), v.state)).collect()
    }

    /// Emits a `ConfigReloaded` event on the kernel broadcast bus.
    pub fn emit_reload(&self, source: impl Into<String>) {
        self.inner.events.emit(KernelEvent::ConfigReloaded {
            source: source.into(),
        });
    }

    /// Registers and mounts (or parks in `Waiting` until dependencies arrive) a root plugin fiber.
    ///
    /// # Errors
    /// Returns `KernelError::DuplicateFiber`, `KernelError::CycleDetected`, or `KernelError::MountFailed`.
    pub async fn mount<P>(
        &self,
        id: impl Into<String>,
        plugin: Arc<P>,
        config: Value,
    ) -> Result<(), KernelError>
    where
        P: Plugin + 'static,
    {
        let _lock = self.inner.lifecycle_lock.lock().await;
        let id_str = id.into();
        let plugin_dyn: Arc<dyn Plugin> = plugin;
        register_fiber_record(&self.inner, id_str, plugin_dyn, config, None)?;
        activate_ready_fibers(&self.inner).await
    }

    /// Disposes fiber `id` along with its child fibers, dependent fibers, and LIFO effects.
    ///
    /// # Errors
    /// Returns `KernelError::FiberNotFound` if `id` is not registered.
    pub async fn dispose(&self, id: &str) -> Result<(), KernelError> {
        let _lock = self.inner.lifecycle_lock.lock().await;
        dispose_fiber_inner(&self.inner, id, true).await
    }

    /// Reconfigures an existing fiber atomically: unwinds its subtree/dependents in LIFO order,
    /// applies `new_config`, and reactively remounts the fiber and waiting dependents.
    ///
    /// On mount failure, restores the fiber's previous config and plugin state.
    ///
    /// # Errors
    /// Returns `KernelError::FiberNotFound` or `KernelError::MountFailed`.
    pub async fn reconfigure(&self, id: &str, new_config: Value) -> Result<(), KernelError> {
        let _lock = self.inner.lifecycle_lock.lock().await;
        let (prev_plugin, prev_config, prev_parent, current_state) = {
            let guard = self.inner.fibers.lock().unwrap_or_else(|e| e.into_inner());
            let rec = guard
                .get(id)
                .ok_or_else(|| KernelError::FiberNotFound(id.to_string()))?;
            (
                Arc::clone(&rec.plugin),
                rec.config.clone(),
                rec.parent_id.clone(),
                rec.state,
            )
        };

        if current_state == FiberState::Active {
            dispose_fiber_inner(&self.inner, id, false).await?;
        }

        {
            let mut guard = self.inner.fibers.lock().unwrap_or_else(|e| e.into_inner());
            let record = guard
                .get_mut(id)
                .ok_or_else(|| KernelError::FiberNotFound(id.to_string()))?;
            record.config = new_config;
        }

        if let Err(err) = activate_ready_fibers(&self.inner).await {
            let still_present = {
                let mut guard = self.inner.fibers.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(record) = guard.get_mut(id) {
                    record.plugin = Arc::clone(&prev_plugin);
                    record.config = prev_config.clone();
                    true
                } else {
                    false
                }
            };
            if !still_present {
                let _ = register_fiber_record(
                    &self.inner,
                    id.to_string(),
                    prev_plugin,
                    prev_config,
                    prev_parent,
                );
            }
            let _ = activate_ready_fibers(&self.inner).await;
            return Err(err);
        }

        Ok(())
    }

    /// Declaratively reconciles active root fibers to match `desired` resolved profile entries.
    ///
    /// Validates all plugin factories and cycle constraints before modifying active fibers, and
    /// rolls back to prior root fiber state if activation fails.
    ///
    /// # Errors
    /// Returns `KernelError` if a factory is unknown, a cycle exists, or a mount fails.
    pub async fn reconcile(
        &self,
        factories: &PluginFactoryRegistry,
        desired: &[ProfileEntry],
    ) -> Result<(), KernelError> {
        let _lock = self.inner.lifecycle_lock.lock().await;

        let enabled_entries: Vec<&ProfileEntry> = desired.iter().filter(|e| e.enabled).collect();

        // Pre-instantiate plugins first so UnknownPlugin fails cleanly before mutating fibers.
        let mut desired_plugins: HashMap<String, (Arc<dyn Plugin>, PluginMeta, &ProfileEntry)> =
            HashMap::new();
        for entry in &enabled_entries {
            if desired_plugins.contains_key(&entry.id) {
                return Err(KernelError::DuplicateFiber(entry.id.clone()));
            }
            let plugin = factories.create(&entry.plugin)?;
            let meta = plugin.meta();
            desired_plugins.insert(entry.id.clone(), (plugin, meta, *entry));
        }

        // Validate full candidate dependency graph for cycles before disposing anything.
        let desired_nodes: Vec<(&str, &PluginMeta)> = desired_plugins
            .iter()
            .map(|(id, (_, meta, _))| (id.as_str(), meta))
            .collect();
        check_graph_cycles(&desired_nodes)?;

        // Snapshot current root fibers (id, plugin, plugin_name, config).
        let existing_roots: Vec<(String, Arc<dyn Plugin>, &'static str, Value)> = {
            let guard = self.inner.fibers.lock().unwrap_or_else(|e| e.into_inner());
            guard
                .values()
                .filter(|f| f.parent_id.is_none())
                .map(|f| {
                    (
                        f.id.clone(),
                        Arc::clone(&f.plugin),
                        f.meta.name,
                        f.config.clone(),
                    )
                })
                .collect()
        };

        // 1. Dispose root fibers removed, disabled, or replaced with a different plugin factory.
        for (id, _, existing_plugin_name, _) in &existing_roots {
            let remove_or_replace = match desired_plugins.get(id) {
                None => true,
                Some((_, meta, _)) => meta.name != *existing_plugin_name,
            };
            if remove_or_replace {
                dispose_fiber_inner(&self.inner, id, true).await?;
            }
        }

        // 2. Reconfigure root fibers whose config changed.
        for (id, _, existing_plugin_name, existing_cfg) in &existing_roots {
            if let Some((_, meta, entry)) = desired_plugins.get(id) {
                if meta.name == *existing_plugin_name && *existing_cfg != entry.config {
                    let state = {
                        let guard = self.inner.fibers.lock().unwrap_or_else(|e| e.into_inner());
                        guard.get(id).map(|f| f.state)
                    };
                    if state == Some(FiberState::Active) {
                        dispose_fiber_inner(&self.inner, id, false).await?;
                    }
                    let mut guard = self.inner.fibers.lock().unwrap_or_else(|e| e.into_inner());
                    if let Some(rec) = guard.get_mut(id) {
                        rec.config = entry.config.clone();
                    }
                }
            }
        }

        // 3. Register newly added desired entries in order and activate.
        let mut step_res: Result<(), KernelError> = Ok(());
        for entry in &enabled_entries {
            let already_present = {
                let guard = self.inner.fibers.lock().unwrap_or_else(|e| e.into_inner());
                guard.contains_key(&entry.id)
            };
            if !already_present {
                if let Some((plugin, _, _)) = desired_plugins.get(&entry.id) {
                    if let Err(err) = register_fiber_record(
                        &self.inner,
                        entry.id.clone(),
                        Arc::clone(plugin),
                        entry.config.clone(),
                        None,
                    ) {
                        step_res = Err(err);
                        break;
                    }
                }
            }
        }
        if step_res.is_ok() {
            step_res = activate_ready_fibers(&self.inner).await;
        }

        if let Err(err) = step_res {
            rollback_reconcile(&self.inner, &existing_roots).await;
            return Err(err);
        }

        Ok(())
    }

    /// Gracefully shuts down the entire kernel in reverse mount order (LIFO).
    pub async fn shutdown(&self) {
        let _lock = self.inner.lifecycle_lock.lock().await;
        self.inner.root_token.cancel();

        let order = {
            let guard = self
                .inner
                .mount_order
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            guard.clone()
        };

        for id in order.into_iter().rev() {
            let still_exists = {
                let guard = self.inner.fibers.lock().unwrap_or_else(|e| e.into_inner());
                guard.contains_key(&id)
            };
            if still_exists {
                let _ = dispose_fiber_inner(&self.inner, &id, true).await;
            }
        }

        // Clean up any remaining Waiting/unmounted fibers.
        let remaining: Vec<String> = {
            let guard = self.inner.fibers.lock().unwrap_or_else(|e| e.into_inner());
            guard.keys().cloned().collect()
        };
        for id in remaining {
            let still_exists = {
                let guard = self.inner.fibers.lock().unwrap_or_else(|e| e.into_inner());
                guard.contains_key(&id)
            };
            if still_exists {
                let _ = dispose_fiber_inner(&self.inner, &id, true).await;
            }
        }
    }
}

impl Default for Kernel {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for Kernel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Kernel")
            .field("fiber_count", &self.fiber_states().len())
            .finish()
    }
}

fn register_fiber_record(
    inner: &Arc<KernelInner>,
    id: String,
    plugin: Arc<dyn Plugin>,
    config: Value,
    parent_id: Option<String>,
) -> Result<(), KernelError> {
    let meta = plugin.meta();

    let mut guard = inner.fibers.lock().unwrap_or_else(|e| e.into_inner());
    if guard.contains_key(&id) {
        return Err(KernelError::DuplicateFiber(id));
    }

    // Check dependency cycles across existing fibers + candidate fiber.
    check_no_cycles(&guard, &id, &meta)?;

    let parent_token = match &parent_id {
        Some(pid) => guard
            .get(pid)
            .map(|p| p.scope.token())
            .unwrap_or_else(|| inner.root_token.clone()),
        None => inner.root_token.clone(),
    };

    let scope = EffectScope::new(&parent_token);
    let plugin_name = meta.name;

    if let Some(pid) = &parent_id {
        if let Some(parent) = guard.get_mut(pid) {
            parent.children.push(id.clone());
        }
    }

    guard.insert(
        id.clone(),
        FiberRecord {
            id: id.clone(),
            plugin,
            meta,
            config,
            parent_id,
            children: Vec::new(),
            scope,
            state: FiberState::Waiting,
        },
    );

    inner.events.emit(KernelEvent::FiberWaiting {
        id,
        plugin: plugin_name,
    });
    Ok(())
}

async fn rollback_reconcile(
    inner: &Arc<KernelInner>,
    prior_roots: &[(String, Arc<dyn Plugin>, &'static str, Value)],
) {
    let prior_ids: HashSet<&str> = prior_roots
        .iter()
        .map(|(id, _, _, _)| id.as_str())
        .collect();
    let extra_roots: Vec<String> = {
        let guard = inner.fibers.lock().unwrap_or_else(|e| e.into_inner());
        guard
            .values()
            .filter(|f| f.parent_id.is_none() && !prior_ids.contains(f.id.as_str()))
            .map(|f| f.id.clone())
            .collect()
    };
    for id in extra_roots {
        let _ = dispose_fiber_inner(inner, &id, true).await;
    }

    for (id, plugin, plugin_name, cfg) in prior_roots {
        let current = {
            let guard = inner.fibers.lock().unwrap_or_else(|e| e.into_inner());
            guard
                .get(id)
                .map(|f| (f.state, f.meta.name, f.config.clone()))
        };
        match current {
            Some((state, cur_name, cur_cfg)) => {
                if cur_name != *plugin_name || cur_cfg != *cfg {
                    if state == FiberState::Active {
                        let _ = dispose_fiber_inner(inner, id, false).await;
                    }
                    let mut guard = inner.fibers.lock().unwrap_or_else(|e| e.into_inner());
                    if let Some(rec) = guard.get_mut(id) {
                        rec.plugin = Arc::clone(plugin);
                        rec.meta = plugin.meta();
                        rec.config = cfg.clone();
                    }
                }
            }
            None => {
                let _ =
                    register_fiber_record(inner, id.clone(), Arc::clone(plugin), cfg.clone(), None);
            }
        }
    }

    let _ = activate_ready_fibers(inner).await;
}

fn check_no_cycles(
    existing: &HashMap<String, FiberRecord>,
    candidate_id: &str,
    candidate_meta: &PluginMeta,
) -> Result<(), KernelError> {
    let mut nodes: Vec<(&str, &PluginMeta)> = existing
        .iter()
        .map(|(fid, rec)| (fid.as_str(), &rec.meta))
        .collect();
    nodes.push((candidate_id, candidate_meta));
    check_graph_cycles(&nodes)
}

fn check_graph_cycles<'a>(nodes: &[(&'a str, &'a PluginMeta)]) -> Result<(), KernelError> {
    // Map TypeId -> all fiber IDs declaring that TypeId in `provides`.
    let mut providers: HashMap<TypeId, Vec<&str>> = HashMap::new();
    let mut requires_map: HashMap<&str, &[TypeId]> = HashMap::new();
    for &(fid, meta) in nodes {
        for &tid in &meta.provides {
            providers.entry(tid).or_default().push(fid);
        }
        requires_map.insert(fid, &meta.requires);
    }

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Color {
        White,
        Gray,
        Black,
    }

    let mut colors: HashMap<&str, Color> =
        requires_map.keys().map(|&k| (k, Color::White)).collect();

    fn dfs<'a>(
        node: &'a str,
        requires_map: &HashMap<&'a str, &[TypeId]>,
        providers: &HashMap<TypeId, Vec<&'a str>>,
        colors: &mut HashMap<&'a str, Color>,
    ) -> Result<(), String> {
        colors.insert(node, Color::Gray);
        if let Some(reqs) = requires_map.get(node) {
            for tid in *reqs {
                if let Some(prov_list) = providers.get(tid) {
                    for &prov in prov_list {
                        match colors.get(prov).copied().unwrap_or(Color::White) {
                            Color::Gray => {
                                return Err(format!("{node} -> {prov}"));
                            }
                            Color::White => dfs(prov, requires_map, providers, colors)?,
                            Color::Black => {}
                        }
                    }
                }
            }
        }
        colors.insert(node, Color::Black);
        Ok(())
    }

    for &node in requires_map.keys() {
        if colors.get(node).copied() == Some(Color::White) {
            dfs(node, &requires_map, &providers, &mut colors)
                .map_err(KernelError::CycleDetected)?;
        }
    }

    Ok(())
}

async fn mount_single_fiber(inner: &Arc<KernelInner>, id: &str) -> Result<(), KernelError> {
    let (plugin, config, scope, plugin_name) = {
        let mut guard = inner.fibers.lock().unwrap_or_else(|e| e.into_inner());
        let rec = guard
            .get_mut(id)
            .ok_or_else(|| KernelError::FiberNotFound(id.to_string()))?;
        rec.state = FiberState::Mounting;
        (
            Arc::clone(&rec.plugin),
            rec.config.clone(),
            rec.scope.clone(),
            rec.meta.name,
        )
    };

    let ctx = Context {
        fiber_id: id.to_string(),
        scope: scope.clone(),
        inner: Arc::clone(inner),
    };

    debug!(fiber_id = %id, plugin = %plugin_name, "mounting Cordis fiber");

    match plugin.mount(ctx, config).await {
        Ok(()) => {
            {
                let mut guard = inner.fibers.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(rec) = guard.get_mut(id) {
                    rec.state = FiberState::Active;
                }
            }
            {
                let mut order = inner.mount_order.lock().unwrap_or_else(|e| e.into_inner());
                if !order.iter().any(|x| x == id) {
                    order.push(id.to_string());
                }
            }
            info!(fiber_id = %id, plugin = %plugin_name, "mounted Cordis fiber");
            inner.events.emit(KernelEvent::FiberMounted {
                id: id.to_string(),
                plugin: plugin_name,
            });
            Ok(())
        }
        Err(err) => {
            // Roll back any children, services, or effects registered prior to failure.
            let children = {
                let mut guard = inner.fibers.lock().unwrap_or_else(|e| e.into_inner());
                let children = guard
                    .get_mut(id)
                    .map(|r| std::mem::take(&mut r.children))
                    .unwrap_or_default();
                if let Some(parent_id) = guard.get(id).and_then(|r| r.parent_id.clone()) {
                    if let Some(parent) = guard.get_mut(&parent_id) {
                        parent.children.retain(|c| c != id);
                    }
                }
                guard.remove(id);
                children
            };
            for child_id in children.into_iter().rev() {
                let _ = dispose_fiber_inner(inner, &child_id, true).await;
            }
            scope.dispose().await;
            inner
                .events
                .emit(KernelEvent::FiberDisposed { id: id.to_string() });
            Err(KernelError::MountFailed {
                fiber_id: id.to_string(),
                reason: err.to_string(),
            })
        }
    }
}

fn activate_ready_fibers<'a>(
    inner: &'a Arc<KernelInner>,
) -> BoxFuture<'a, Result<(), KernelError>> {
    Box::pin(async move {
        loop {
            let next_ready: Option<String> = {
                let guard = inner.fibers.lock().unwrap_or_else(|e| e.into_inner());
                let mut candidates: Vec<&FiberRecord> = guard
                    .values()
                    .filter(|rec| {
                        rec.state == FiberState::Waiting
                            && rec
                                .meta
                                .requires
                                .iter()
                                .all(|req| inner.registry.contains(*req))
                    })
                    .collect();
                candidates.sort_by(|a, b| a.id.cmp(&b.id));
                candidates.first().map(|rec| rec.id.clone())
            };

            let Some(fiber_id) = next_ready else {
                break;
            };
            mount_single_fiber(inner, &fiber_id).await?;
        }
        Ok(())
    })
}

pub(crate) async fn fork_child_inner(
    inner: &Arc<KernelInner>,
    child_id: String,
    plugin: Arc<dyn Plugin>,
    config: Value,
    parent_id: &str,
) -> Result<(), KernelError> {
    register_fiber_record(
        inner,
        child_id.clone(),
        plugin,
        config,
        Some(parent_id.to_string()),
    )?;

    let deps_ready = {
        let guard = inner.fibers.lock().unwrap_or_else(|e| e.into_inner());
        guard.get(&child_id).is_some_and(|rec| {
            rec.meta
                .requires
                .iter()
                .all(|req| inner.registry.contains(*req))
        })
    };

    if deps_ready {
        mount_single_fiber(inner, &child_id).await?;
    }
    Ok(())
}

fn dispose_fiber_inner<'a>(
    inner: &'a Arc<KernelInner>,
    id: &'a str,
    remove_record: bool,
) -> BoxFuture<'a, Result<(), KernelError>> {
    Box::pin(async move {
        let (children, provides, scope, parent_id, was_active) = {
            let mut guard = inner.fibers.lock().unwrap_or_else(|e| e.into_inner());
            let rec = guard
                .get_mut(id)
                .ok_or_else(|| KernelError::FiberNotFound(id.to_string()))?;
            if rec.state == FiberState::Disposing {
                return Ok(());
            }
            let was_active = rec.state == FiberState::Active;
            rec.state = FiberState::Disposing;
            (
                rec.children.clone(),
                rec.meta.provides.clone(),
                rec.scope.clone(),
                rec.parent_id.clone(),
                was_active,
            )
        };

        // 1. Dispose child fibers first (LIFO).
        for child_id in children.into_iter().rev() {
            let exists = {
                let guard = inner.fibers.lock().unwrap_or_else(|e| e.into_inner());
                guard.contains_key(&child_id)
            };
            if exists {
                dispose_fiber_inner(inner, &child_id, true).await?;
            }
        }

        // 2. Suspend active sibling/downstream fibers that depend on services provided by `id`.
        if was_active && !provides.is_empty() {
            let provided_set: HashSet<TypeId> = provides.into_iter().collect();
            let dependents: Vec<String> = {
                let guard = inner.fibers.lock().unwrap_or_else(|e| e.into_inner());
                let other_providers: HashSet<TypeId> = guard
                    .values()
                    .filter(|rec| rec.id != id && rec.state == FiberState::Active)
                    .flat_map(|rec| rec.meta.provides.iter().copied())
                    .collect();
                guard
                    .values()
                    .filter(|rec| {
                        rec.id != id
                            && rec.state == FiberState::Active
                            && rec.meta.requires.iter().any(|req| {
                                provided_set.contains(req) && !other_providers.contains(req)
                            })
                    })
                    .map(|rec| rec.id.clone())
                    .collect()
            };

            for dep_id in dependents {
                dispose_fiber_inner(inner, &dep_id, false).await?;
            }
        }

        // 3. Unwind `EffectScope` in LIFO order (removes services, effects, and cancels tasks).
        scope.dispose().await;

        // 4. Update fiber table & mount_order.
        {
            let mut order = inner.mount_order.lock().unwrap_or_else(|e| e.into_inner());
            order.retain(|x| x != id);
        }

        {
            let mut guard = inner.fibers.lock().unwrap_or_else(|e| e.into_inner());
            if remove_record {
                if let Some(pid) = parent_id {
                    if let Some(parent) = guard.get_mut(&pid) {
                        parent.children.retain(|c| c != id);
                    }
                }
                guard.remove(id);
            } else {
                let parent_token = parent_id
                    .as_deref()
                    .and_then(|pid| guard.get(pid))
                    .map(|p| p.scope.token())
                    .unwrap_or_else(|| inner.root_token.clone());
                if let Some(rec) = guard.get_mut(id) {
                    rec.children.clear();
                    rec.scope = EffectScope::new(&parent_token);
                    rec.state = FiberState::Waiting;
                }
            }
        }

        if remove_record {
            info!(fiber_id = %id, "disposed Cordis fiber");
            inner
                .events
                .emit(KernelEvent::FiberDisposed { id: id.to_string() });
        } else {
            debug!(fiber_id = %id, "suspended Cordis fiber to Waiting");
        }

        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::reconcile::{resolve_profile, CordisConfig, CordisPatch, PatchEntry};
    use serde_json::json;
    use std::sync::atomic::{AtomicBool, Ordering};

    struct SvcA(i32);
    struct SvcB(i32);

    struct RecordingPlugin {
        meta: PluginMeta,
        log: Arc<Mutex<Vec<String>>>,
        fail_mount: bool,
        provide_a: Option<i32>,
        provide_b: Option<i32>,
    }

    impl Plugin for RecordingPlugin {
        fn meta(&self) -> PluginMeta {
            self.meta.clone()
        }

        fn mount<'a>(
            &'a self,
            ctx: Context,
            config: Value,
        ) -> BoxFuture<'a, Result<(), KernelError>> {
            Box::pin(async move {
                let fid = ctx.fiber_id().to_string();
                self.log
                    .lock()
                    .unwrap()
                    .push(format!("mount:{fid}:{config}"));

                if let Some(v) = self.provide_a {
                    ctx.provide(Arc::new(SvcA(v)));
                }
                if let Some(v) = self.provide_b {
                    ctx.provide(Arc::new(SvcB(v)));
                }

                let log_ref = Arc::clone(&self.log);
                let fid_e1 = fid.clone();
                ctx.on_dispose_sync(move || {
                    log_ref.lock().unwrap().push(format!("dispose:{fid_e1}:e1"));
                });

                let log_ref2 = Arc::clone(&self.log);
                let fid_e2 = fid.clone();
                ctx.on_dispose(move || async move {
                    log_ref2
                        .lock()
                        .unwrap()
                        .push(format!("dispose:{fid_e2}:e2"));
                });

                if self.fail_mount {
                    return Err(KernelError::Internal("boom".to_string()));
                }
                Ok(())
            })
        }
    }

    #[tokio::test]
    async fn test_lifo_effect_disposal_and_task_cancellation() {
        let kernel = Kernel::new();
        let log = Arc::new(Mutex::new(Vec::new()));
        let cancelled = Arc::new(AtomicBool::new(false));
        let cancelled_flag = Arc::clone(&cancelled);

        struct TaskPlugin {
            log: Arc<Mutex<Vec<String>>>,
            cancelled: Arc<AtomicBool>,
        }
        impl Plugin for TaskPlugin {
            fn meta(&self) -> PluginMeta {
                PluginMeta::new("task_plugin").provides::<SvcA>()
            }
            fn mount<'a>(
                &'a self,
                ctx: Context,
                _config: Value,
            ) -> BoxFuture<'a, Result<(), KernelError>> {
                Box::pin(async move {
                    ctx.provide(Arc::new(SvcA(42)));
                    let l1 = Arc::clone(&self.log);
                    ctx.on_dispose_sync(move || l1.lock().unwrap().push("E1".into()));
                    let l2 = Arc::clone(&self.log);
                    ctx.on_dispose(move || async move { l2.lock().unwrap().push("E2".into()) });
                    let l3 = Arc::clone(&self.log);
                    ctx.on_dispose_sync(move || l3.lock().unwrap().push("E3".into()));

                    let flag = Arc::clone(&self.cancelled);
                    ctx.spawn("worker", move |tok| async move {
                        tok.cancelled().await;
                        flag.store(true, Ordering::SeqCst);
                    });
                    Ok(())
                })
            }
        }

        kernel
            .mount(
                "worker-fiber",
                Arc::new(TaskPlugin {
                    log: Arc::clone(&log),
                    cancelled: cancelled_flag,
                }),
                json!({}),
            )
            .await
            .unwrap();

        assert_eq!(kernel.inject::<SvcA>().unwrap().0, 42);
        kernel.dispose("worker-fiber").await.unwrap();

        assert!(cancelled.load(Ordering::SeqCst));
        assert!(kernel.inject::<SvcA>().is_none());
        assert_eq!(*log.lock().unwrap(), vec!["E3", "E2", "E1"]);
    }

    #[tokio::test]
    async fn test_reactive_out_of_order_mount_and_cascade_suspend() {
        let kernel = Kernel::new();
        let log = Arc::new(Mutex::new(Vec::new()));

        let consumer = Arc::new(RecordingPlugin {
            meta: PluginMeta::new("consumer")
                .requires::<SvcA>()
                .provides::<SvcB>(),
            log: Arc::clone(&log),
            fail_mount: false,
            provide_a: None,
            provide_b: Some(99),
        });
        let provider = Arc::new(RecordingPlugin {
            meta: PluginMeta::new("provider").provides::<SvcA>(),
            log: Arc::clone(&log),
            fail_mount: false,
            provide_a: Some(7),
            provide_b: None,
        });

        // Mount consumer before provider -> consumer parks in Waiting.
        kernel.mount("consumer", consumer, json!({})).await.unwrap();
        assert_eq!(kernel.fiber_state("consumer"), Some(FiberState::Waiting));
        assert!(kernel.inject::<SvcB>().is_none());

        // Mount provider -> consumer mounts automatically!
        kernel
            .mount("provider", provider, json!({"v": 1}))
            .await
            .unwrap();
        assert_eq!(kernel.fiber_state("provider"), Some(FiberState::Active));
        assert_eq!(kernel.fiber_state("consumer"), Some(FiberState::Active));
        assert_eq!(kernel.inject::<SvcB>().unwrap().0, 99);

        // Reconfigure provider -> consumer suspends (disposes e2 -> e1) before provider disposes, then remounts!
        kernel
            .reconfigure("provider", json!({"v": 2}))
            .await
            .unwrap();
        assert_eq!(kernel.fiber_state("consumer"), Some(FiberState::Active));
        assert_eq!(kernel.inject::<SvcB>().unwrap().0, 99);

        // Dispose provider permanently -> consumer suspends cleanly to Waiting and removes SvcB.
        kernel.dispose("provider").await.unwrap();
        assert_eq!(kernel.fiber_state("provider"), None);
        assert_eq!(kernel.fiber_state("consumer"), Some(FiberState::Waiting));
        assert!(kernel.inject::<SvcA>().is_none());
        assert!(kernel.inject::<SvcB>().is_none());
    }

    #[tokio::test]
    async fn test_cycle_detection_and_failed_mount_rollback() {
        let kernel = Kernel::new();
        let log = Arc::new(Mutex::new(Vec::new()));

        let a = Arc::new(RecordingPlugin {
            meta: PluginMeta::new("a").provides::<SvcA>().requires::<SvcB>(),
            log: Arc::clone(&log),
            fail_mount: false,
            provide_a: Some(1),
            provide_b: None,
        });
        let b_cyclic = Arc::new(RecordingPlugin {
            meta: PluginMeta::new("b").provides::<SvcB>().requires::<SvcA>(),
            log: Arc::clone(&log),
            fail_mount: false,
            provide_a: None,
            provide_b: Some(2),
        });

        kernel.mount("a", a, json!({})).await.unwrap();
        let err = kernel.mount("b", b_cyclic, json!({})).await.unwrap_err();
        assert!(matches!(err, KernelError::CycleDetected(_)));

        let failing = Arc::new(RecordingPlugin {
            meta: PluginMeta::new("failing").provides::<SvcB>(),
            log: Arc::clone(&log),
            fail_mount: true,
            provide_a: None,
            provide_b: Some(123),
        });
        let mount_err = kernel
            .mount("failing", failing, json!({}))
            .await
            .unwrap_err();
        assert!(matches!(mount_err, KernelError::MountFailed { .. }));
        assert_eq!(kernel.fiber_state("failing"), None);
        assert!(kernel.inject::<SvcB>().is_none());
    }

    #[tokio::test]
    async fn test_duplicate_and_missing_fiber_errors() {
        let kernel = Kernel::new();
        let log = Arc::new(Mutex::new(Vec::new()));
        let p = Arc::new(RecordingPlugin {
            meta: PluginMeta::new("p"),
            log,
            fail_mount: false,
            provide_a: None,
            provide_b: None,
        });
        kernel.mount("p1", Arc::clone(&p), json!({})).await.unwrap();
        assert!(matches!(
            kernel.mount("p1", p, json!({})).await.unwrap_err(),
            KernelError::DuplicateFiber(_)
        ));
        assert!(matches!(
            kernel.dispose("missing").await.unwrap_err(),
            KernelError::FiberNotFound(_)
        ));
    }

    #[tokio::test]
    async fn test_profile_patch_and_reconcile() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut factories = PluginFactoryRegistry::new();
        let log1 = Arc::clone(&log);
        factories.register("prov", move || {
            Arc::new(RecordingPlugin {
                meta: PluginMeta::new("prov").provides::<SvcA>(),
                log: Arc::clone(&log1),
                fail_mount: false,
                provide_a: Some(10),
                provide_b: None,
            })
        });
        let log2 = Arc::clone(&log);
        factories.register("cons", move || {
            Arc::new(RecordingPlugin {
                meta: PluginMeta::new("cons")
                    .requires::<SvcA>()
                    .provides::<SvcB>(),
                log: Arc::clone(&log2),
                fail_mount: false,
                provide_a: None,
                provide_b: Some(20),
            })
        });

        let cfg = CordisConfig {
            profiles: HashMap::from([(
                "serve".to_string(),
                vec![
                    ProfileEntry {
                        id: "p".into(),
                        plugin: "prov".into(),
                        enabled: true,
                        config: json!({"a": 1, "b": 2}),
                    },
                    ProfileEntry {
                        id: "c".into(),
                        plugin: "cons".into(),
                        enabled: true,
                        config: json!({}),
                    },
                ],
            )]),
        };
        let patch = CordisPatch {
            patches: vec![PatchEntry {
                id: "p".into(),
                plugin: None,
                enabled: None,
                config: Some(json!({"b": 99})),
            }],
        };

        let resolved = resolve_profile(&cfg, Some(&patch), "serve").unwrap();
        assert_eq!(resolved[0].config, json!({"a": 1, "b": 99}));

        let kernel = Kernel::new();
        kernel.reconcile(&factories, &resolved).await.unwrap();
        assert_eq!(kernel.fiber_state("p"), Some(FiberState::Active));
        assert_eq!(kernel.fiber_state("c"), Some(FiberState::Active));

        // Disable consumer via patch and reconcile live.
        let disable_patch = CordisPatch {
            patches: vec![PatchEntry {
                id: "c".into(),
                plugin: None,
                enabled: Some(false),
                config: None,
            }],
        };
        let resolved2 = resolve_profile(&cfg, Some(&disable_patch), "serve").unwrap();
        kernel.reconcile(&factories, &resolved2).await.unwrap();
        assert_eq!(kernel.fiber_state("p"), Some(FiberState::Active));
        assert_eq!(kernel.fiber_state("c"), None);
        assert!(kernel.inject::<SvcB>().is_none());

        kernel.shutdown().await;
        assert!(kernel.fiber_states().is_empty());
    }

    #[tokio::test]
    async fn test_child_fiber_fork_and_lifo_parent_teardown() {
        let kernel = Kernel::new();
        let log = Arc::new(Mutex::new(Vec::new()));

        struct ParentPlugin {
            log: Arc<Mutex<Vec<String>>>,
        }
        impl Plugin for ParentPlugin {
            fn meta(&self) -> PluginMeta {
                PluginMeta::new("parent")
            }
            fn mount<'a>(
                &'a self,
                ctx: Context,
                _config: Value,
            ) -> BoxFuture<'a, Result<(), KernelError>> {
                Box::pin(async move {
                    let l1 = Arc::clone(&self.log);
                    ctx.on_dispose_sync(move || l1.lock().unwrap().push("parent_e1".into()));

                    let child = Arc::new(RecordingPlugin {
                        meta: PluginMeta::new("child").provides::<SvcA>(),
                        log: Arc::clone(&self.log),
                        fail_mount: false,
                        provide_a: Some(55),
                        provide_b: None,
                    });
                    ctx.fork("parent::child", child, json!({})).await?;
                    Ok(())
                })
            }
        }

        kernel
            .mount(
                "parent",
                Arc::new(ParentPlugin {
                    log: Arc::clone(&log),
                }),
                json!({}),
            )
            .await
            .unwrap();

        assert_eq!(kernel.fiber_state("parent"), Some(FiberState::Active));
        assert_eq!(
            kernel.fiber_state("parent::child"),
            Some(FiberState::Active)
        );
        assert_eq!(kernel.inject::<SvcA>().unwrap().0, 55);

        kernel.dispose("parent").await.unwrap();
        assert_eq!(kernel.fiber_state("parent::child"), None);
        assert!(kernel.inject::<SvcA>().is_none());

        let events = log.lock().unwrap().clone();
        // Child disposes BEFORE parent's effect (`parent_e1`), and child's effects unwind LIFO (e2 -> e1).
        let child_e2 = events
            .iter()
            .position(|e| e == "dispose:parent::child:e2")
            .unwrap();
        let child_e1 = events
            .iter()
            .position(|e| e == "dispose:parent::child:e1")
            .unwrap();
        let parent_e1 = events.iter().position(|e| e == "parent_e1").unwrap();
        assert!(child_e2 < child_e1 && child_e1 < parent_e1);
    }

    #[tokio::test]
    async fn test_effect_scope_joins_tasks_before_running_disposers() {
        let kernel = Kernel::new();
        let log = Arc::new(Mutex::new(Vec::new()));

        struct OrderPlugin {
            log: Arc<Mutex<Vec<String>>>,
        }
        impl Plugin for OrderPlugin {
            fn meta(&self) -> PluginMeta {
                PluginMeta::new("order_plugin")
            }
            fn mount<'a>(
                &'a self,
                ctx: Context,
                _config: Value,
            ) -> BoxFuture<'a, Result<(), KernelError>> {
                Box::pin(async move {
                    let disp_log = Arc::clone(&self.log);
                    ctx.on_dispose_sync(move || {
                        disp_log.lock().unwrap().push("disposer_ran".to_string());
                    });

                    let task_log = Arc::clone(&self.log);
                    ctx.spawn("ordered_worker", move |tok| async move {
                        tok.cancelled().await;
                        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                        task_log.lock().unwrap().push("task_stopped".to_string());
                    });
                    Ok(())
                })
            }
        }

        kernel
            .mount(
                "order",
                Arc::new(OrderPlugin {
                    log: Arc::clone(&log),
                }),
                json!({}),
            )
            .await
            .unwrap();

        kernel.dispose("order").await.unwrap();
        let events = log.lock().unwrap().clone();
        assert_eq!(events, vec!["task_stopped", "disposer_ran"]);
    }

    #[tokio::test]
    async fn test_reconcile_rejects_cycle_without_disposing_active_fibers() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut factories = PluginFactoryRegistry::new();
        let log_k = Arc::clone(&log);
        factories.register("keep", move || {
            Arc::new(RecordingPlugin {
                meta: PluginMeta::new("keep").provides::<SvcA>(),
                log: Arc::clone(&log_k),
                fail_mount: false,
                provide_a: Some(77),
                provide_b: None,
            })
        });
        let log_a = Arc::clone(&log);
        factories.register("cyc_a", move || {
            Arc::new(RecordingPlugin {
                meta: PluginMeta::new("cyc_a")
                    .provides::<SvcA>()
                    .requires::<SvcB>(),
                log: Arc::clone(&log_a),
                fail_mount: false,
                provide_a: Some(1),
                provide_b: None,
            })
        });
        let log_b = Arc::clone(&log);
        factories.register("cyc_b", move || {
            Arc::new(RecordingPlugin {
                meta: PluginMeta::new("cyc_b")
                    .provides::<SvcB>()
                    .requires::<SvcA>(),
                log: Arc::clone(&log_b),
                fail_mount: false,
                provide_a: None,
                provide_b: Some(2),
            })
        });

        let kernel = Kernel::new();
        kernel
            .reconcile(
                &factories,
                &[ProfileEntry {
                    id: "keep".into(),
                    plugin: "keep".into(),
                    enabled: true,
                    config: json!({}),
                }],
            )
            .await
            .unwrap();
        assert_eq!(kernel.fiber_state("keep"), Some(FiberState::Active));
        assert_eq!(kernel.inject::<SvcA>().unwrap().0, 77);

        let err = kernel
            .reconcile(
                &factories,
                &[
                    ProfileEntry {
                        id: "cyc_a".into(),
                        plugin: "cyc_a".into(),
                        enabled: true,
                        config: json!({}),
                    },
                    ProfileEntry {
                        id: "cyc_b".into(),
                        plugin: "cyc_b".into(),
                        enabled: true,
                        config: json!({}),
                    },
                ],
            )
            .await
            .unwrap_err();
        assert!(matches!(err, KernelError::CycleDetected(_)));
        assert_eq!(kernel.fiber_state("keep"), Some(FiberState::Active));
        assert_eq!(kernel.inject::<SvcA>().unwrap().0, 77);

        kernel.shutdown().await;
    }

    #[tokio::test]
    async fn test_reconfigure_rollback_restores_previous_config_on_mount_failure() {
        struct TogglePlugin;
        impl Plugin for TogglePlugin {
            fn meta(&self) -> PluginMeta {
                PluginMeta::new("toggle").provides::<SvcA>()
            }
            fn mount<'a>(
                &'a self,
                ctx: Context,
                config: Value,
            ) -> BoxFuture<'a, Result<(), KernelError>> {
                Box::pin(async move {
                    if config.get("fail").and_then(|v| v.as_bool()) == Some(true) {
                        return Err(KernelError::Internal("injected mount error".into()));
                    }
                    let v = config.get("v").and_then(|x| x.as_i64()).unwrap_or(0) as i32;
                    ctx.provide(Arc::new(SvcA(v)));
                    Ok(())
                })
            }
        }

        let kernel = Kernel::new();
        kernel
            .mount("toggle", Arc::new(TogglePlugin), json!({"v": 10}))
            .await
            .unwrap();
        assert_eq!(kernel.inject::<SvcA>().unwrap().0, 10);

        let err = kernel
            .reconfigure("toggle", json!({"fail": true}))
            .await
            .unwrap_err();
        assert!(matches!(err, KernelError::MountFailed { .. }));
        assert_eq!(kernel.fiber_state("toggle"), Some(FiberState::Active));
        assert_eq!(kernel.inject::<SvcA>().unwrap().0, 10);

        kernel.shutdown().await;
    }
}
