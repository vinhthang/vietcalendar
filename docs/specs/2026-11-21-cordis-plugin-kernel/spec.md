# Cordis-Style Non-Blocking Plugin Kernel and Profiles

- Feature ID: 2026-11-21-cordis-plugin-kernel
- Type: new feature
- Status: In-Testing
- First entered In-Testing: 2026-11-21
- Related specifications: None

## Problem and source intent

User request: "review architecture, I want a clean, none blocking, and performance focus, and a plugin architecture like cordis in deepseek harness used" — followed by `/grill-me` decisions settling on a native Rust Cordis kernel inside `vietcalendar-rs` with `Context`, `Fiber` lifecycle tree, reactive `provide`/`inject`, LIFO `ctx.effect()` disposers, wait-free `ArcSwap` registries, and YAML Profiles/Patches (`config/cordis.yml`, `config/cordis.patch.yml`) with live `notify`/`SIGHUP` hot-reload.

## Repository grounding

- Current runtime ([`src/main.rs`](../../../src/main.rs)) hardcodes telemetry, Axum routes, Swagger UI, and MCP transports with no plugin seam, no reversible lifecycle unwinding, and no runtime reconfiguration.
- [`docs/RUST_STYLE_GUIDE.md`](../../RUST_STYLE_GUIDE.md) mandates zero `.unwrap()`/`.expect()` on production paths, clean layer boundaries, and `///` rustdoc.

## Non-goals

- Loading external dynamic `.so`/`.dll` C-ABI libraries or `wasmtime` WASM guest modules (settled: native compiled-in Rust plugins only).
- Importing any VietCalendar domain types (`LunarDate`, `axum`, `chrono`) inside `src/kernel/`.

## Requirements

### REQ-022: Zero-Domain Cordis Kernel, Context, and Fiber Lifecycle Tree

- Source intent: Settled `/grill-me` architecture decision ("Native Rust Cordis Kernel: compiled-in plugins with full runtime Context, Fiber tree...").
- Statement: WHEN mounting, disposing, reconfiguring, or shutting down plugins, THE Cordis kernel (`src/kernel/`) SHALL manage hierarchical `Fiber` nodes with scoped `Context` handles, enforce zero domain coupling, and cancel all `ctx.spawn` background tasks cleanly when a fiber disposes or shuts down.
- AC-022-1 — Success: `Kernel::mount`, `Kernel::dispose`, `Kernel::reconfigure`, and `Kernel::shutdown` execute without domain dependencies (`src/kernel/` imports no `contracts`, `plugins`, `axum`, or `chrono`), and `ctx.spawn` tasks observe cancellation tokens upon fiber disposal or kernel shutdown with zero leaked tasks.
- AC-022-2 — Boundary/failure: IF mounting a duplicate fiber ID or disposing an unknown fiber ID is attempted, THEN the kernel SHALL return a structured `KernelError` without panicking or corrupting active sibling fibers.

### REQ-023: Wait-Free `ArcSwap` Service Registry and Reactive Dependency Resolution

- Source intent: Cordis spatial composability (`provide` / `inject`) paired with lock-free read-side performance ("wait-free ArcSwap hot-swappable registries").
- Statement: WHEN plugins provide or require typed services (`TypeId`), THE service registry SHALL expose wait-free read lookups via `ArcSwap` snapshots, automatically transition `Waiting` fibers to `Active` when required services appear, detect dependency cycles, and cascade-suspend or unwind dependent fibers when a required provider disposes.
- AC-023-1 — Success: Out-of-order plugin registration (`Consumer` mounted before `Provider`) leaves `Consumer` in `Waiting` and automatically mounts `Consumer` once `Provider` provides its service; reading via `ctx.inject::<S>()` performs a lock-free `ArcSwap` load without acquiring a mutex or `RwLock`.
- AC-023-2 — Boundary/failure: IF a plugin declares cyclic `provides`/`requires` dependencies or requires a service that is later withdrawn, THEN cycle registration returns `KernelError::CycleDetected` and provider disposal suspends/unwinds dependent fibers cleanly before continuing teardown.

### REQ-024: Reversible Temporal Effects and LIFO Unwinding

- Source intent: Cordis temporal composability ("LIFO ctx.effect() / on_dispose unwinders").
- Statement: WHEN a plugin registers side effects via `ctx.effect`, `ctx.on_dispose`, or `ctx.on_dispose_sync`, THE fiber `EffectScope` SHALL record cleanup callbacks and invoke them strictly in last-in, first-out (LIFO) order upon disposal, mount rollback, reconfiguration, or shutdown.
- AC-024-1 — Success: Disposing a fiber that registered effects `[E1, E2, E3]` executes cleanup hooks strictly in reverse order `[E3, E2, E1]`, and `reconfigure` runs full LIFO teardown before remounting with updated configuration.
- AC-024-2 — Boundary/failure: IF a plugin fails mid-`mount` after registering partial services or effects, THEN the kernel SHALL immediately roll back already-registered effects and services in LIFO order and leave the registry clean.

### REQ-025: Declarative YAML Profiles, Bundles, and Patch Reconciliation

- Source intent: DeepSeek Harness profile/bundle/patch pattern settled in `/grill-me` ("YAML config files `cordis.yml` + `cordis.patch.yml`").
- Statement: WHEN loading a runtime profile (`serve`, `mcp-stdio`, `http-only`) from `config/cordis.yml` and optional overrides from `config/cordis.patch.yml`, THE reconciler (`src/profiles/`) SHALL deep-merge patches by plugin ID, evaluate `enabled` flags, and reconcile live `Kernel` fibers declaratively (unmounting disabled/removed entries, reconfiguring changed configs, and mounting added entries).
- AC-025-1 — Success: Resolving `serve`, `mcp-stdio`, and `http-only` profiles produces deterministic plugin sets; applying a `cordis.patch.yml` override toggles `enabled` or deep-merges `config` fields cleanly and reconciles running fibers live.
- AC-025-2 — Boundary/failure: IF an unknown profile name is requested or a patch targets an unknown plugin plugin factory, THEN reconciliation SHALL return a descriptive `KernelError` without mutating or tearing down already-healthy unrelated fibers.

### REQ-026: Non-Blocking Live Hot-Reload (`notify` and Unix `SIGHUP`)

- Source intent: Settled `/grill-me` configuration reload decision ("non-blocking `notify` OS file-watcher... and Unix `SIGHUP` hot-reload with atomic fail-safe validation").
- Statement: WHEN YAML files under the configuration directory (`cordis.yml`, `cordis.patch.yml`, `holidays/*.yml`) change on disk or Unix `SIGHUP` arrives, THE `ConfigReloaderPlugin` SHALL debounce filesystem notifications over a bounded `tokio::sync::mpsc` channel, parse and validate new YAML off the hot request path, and atomically update runtime state.
- AC-026-1 — Success: Valid edits to `cordis.patch.yml` or `holidays/<year>.yml` automatically reconcile kernel plugins or replace active holiday schedules within the debounce window without dropping active HTTP/MCP requests.
- AC-026-2 — Boundary/failure: IF modified YAML is malformed or fails schema validation, THEN the reloader SHALL log an error and preserve the last known-good kernel configuration and holiday schedules intact.

## Assumptions and owner-deferred questions

- **Single-crate assumption:** Compiling `kernel`, `contracts`, `plugins`, and `profiles` inside crate `vietcalendar-rs` keeps Docker build times and release packaging simple; extracting `kernel` into its own workspace crate later remains straightforward via module re-exports.
- **No owner-deferred questions:** All Cordis runtime, YAML format, and hot-reload choices were settled during `/grill-me`.

## Approvals and lifecycle history

- 2026-11-21 (`Planning -> In-Progress`): Authorized via `/goal` and `/grill-me` scope decisions ("End-to-end redesign: write spec-driven architecture & delta specs, implement the Cordis-style non-blocking plugin kernel + refactored plugins...") covering `REQ-022..REQ-026` after Gate 1 and Gate 2 passed.
- 2026-11-21 (`In-Progress -> In-Testing`): [Task 1](./tasks.md#task-1), [Task 2](./tasks.md#task-2), and [Run 1](./verification.md#run-1--2026-11-21) passed; awaiting human real-world usage confirmation before promoting to `Verified`.
