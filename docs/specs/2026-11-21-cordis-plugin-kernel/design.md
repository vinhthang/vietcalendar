# Design: Cordis-Style Non-Blocking Plugin Kernel and Profiles

Sources: [spec.md](./spec.md), [decisions.md](./decisions.md)

## Components

| Requirement / AC | Path and symbol | Responsibility / change |
| --- | --- | --- |
| REQ-022 / AC-022-1, AC-022-2 | `src/kernel/mod.rs`, `src/kernel/error.rs`, `src/kernel/fiber.rs`, `src/kernel/context.rs` | Zero-domain `Kernel`, `Context`, `Fiber`, `Plugin` trait, lifecycle-bound `ctx.spawn`, `mount`, `dispose`, `reconfigure`, `shutdown`. |
| REQ-023 / AC-023-1, AC-023-2 | `src/kernel/registry.rs`, `src/kernel/event.rs` | Wait-free `ArcSwap` `ServiceRegistry` (`provide`, `inject`, `try_inject`), reactive dependency matching, cycle detection, cascading disposal, `EventBus`. |
| REQ-024 / AC-024-1, AC-024-2 | `src/kernel/effect.rs` | `EffectScope` storing sync/async disposers and cancellation tokens, strictly unwinding LIFO on dispose or failed-mount rollback. |
| REQ-025 / AC-025-1, AC-025-2 | `src/kernel/reconcile.rs`, `src/profiles/mod.rs`, `config/cordis.yml`, `config/cordis.patch.yml` | YAML profile/patch deserialization, JSON deep-merge overrides, `PluginFactoryRegistry`, and declarative `Kernel::reconcile`. |
| REQ-026 / AC-026-1, AC-026-2 | `src/plugins/config_reloader.rs` | `notify` OS file-watcher bridged over bounded `mpsc` + 200ms debounce + Unix `SIGHUP` with atomic fail-safe YAML validation. |

## Interfaces and flow

- **Kernel & Fiber Tree:** `Kernel::new()` owns an `Arc<KernelInner>` containing `ArcSwap<HashMap<TypeId, ServiceEntry>>`, a fiber table (`Mutex<HashMap<String, FiberRecord>>`), `EventBus`, and root `CancellationToken`. Each `Fiber` owns a `Context`, child fiber IDs, provided `TypeId`s, required `TypeId`s, `EffectScope`, and `FiberState` (`Waiting`, `Mounting`, `Active`, `Disposing`).
- **Reactive `provide` / `inject`:** Calling `ctx.provide::<S>(arc_s)` publishes `Arc<dyn Any + Send + Sync>` via `ArcSwap::store` (wait-free reads on `ctx.inject::<S>()`), records rollback removal in `EffectScope`, and checks `Waiting` fibers whose `requires` are now satisfied.
- **Reconciliation (`Kernel::reconcile`):** Resolves `CordisConfig` + `CordisPatch` for the active profile; computes diff against active root fibers: unmounts removed/disabled IDs (cascading dependencies), reconfigures changed configs, and mounts new/enabled plugins.

## Boundaries and recovery

- **Zero domain leakage:** `src/kernel/` depends solely on `std`, `tokio`, `tokio-util`, `arc-swap`, `serde`, `serde_json`, and `tracing`.
- **Failed mount rollback:** Any `Err(KernelError)` returned from `Plugin::mount` runs `EffectScope::dispose()` in LIFO order immediately, removing partial services/disposers and setting no dangling `Active` fiber.
- **Fail-safe reload:** `ConfigReloaderPlugin` parses and validates YAML into staging values before touching `HolidayService` or calling `Kernel::reconcile`; invalid YAML logs a structured warning and leaves existing state active.

## Dependencies and unresolved design work

None.
