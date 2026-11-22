# Decisions: Cordis-Style Non-Blocking Plugin Kernel and Profiles

Governing specification: [spec.md](./spec.md)

## Architectural rationale

- Governs: REQ-022, REQ-023, REQ-024, REQ-025, REQ-026

Cordis's core guarantee is **spatiotemporal composability**: plugins declare typed dependencies (`provides`/`requires`), register reversible side effects (`EffectScope`), and mount/dispose dynamically as declarative YAML profiles (`cordis.yml` + `cordis.patch.yml`) reconcile. Mapping this into Rust without blocking request hot paths pairs `ArcSwap` wait-free service reads with `tokio::sync` bounded channels and `CancellationToken` task lifetimes. See `ADR-007` and `ADR-008`.

## ADR-007: Native Compiled-In Rust Cordis Kernel with `ArcSwap` Registries and LIFO `EffectScope`

- Governs: REQ-022, REQ-023, REQ-024, REQ-026
- State: Accepted
- Context and constraints: DeepSeek Harness implements Cordis in TypeScript using dynamic module loading and prototype/context proxies. Rust has no runtime dynamic class mutation, and VietCalendar targets sub-millisecond non-blocking HTTP and MCP throughput.
- Viable options: Option A — Native compiled-in Rust `Kernel` (`TypeId` service registry backed by `ArcSwap`, `Fiber` tree, `EffectScope` LIFO disposers, `PluginFactoryRegistry` driven by YAML config/hot-reload); vs. Option B — Embed a `wasmtime`/WIT WebAssembly runtime for dynamic `.wasm` plugins; vs. Option C — Boot-time-only static plugin wiring with no runtime disposal or live reconfiguration.
- Decision: Option A — Native compiled-in Rust Cordis Kernel with runtime `Fiber` tree, `ArcSwap` registries, LIFO `EffectScope`, and live YAML reconciliation.
- Costs and risks: Preserves zero-overhead direct Rust trait/Arc calls on every request while delivering full Cordis semantics (out-of-order reactive mounting, reversible route/tool/schedule registration, cascading disposal, and live YAML hot-reload). Adding brand-new third-party plugins requires recompiling the binary, which fits a self-contained Rust web/MCP service.
- Rejected alternatives: Option B introduces WASM host serialization latency and large binary/dependency overhead; Option C forfeits live `reconfigure`, YAML patch hot-reload, and reversible side-effect verification.
- Affected components: `src/kernel/*`, `src/profiles/mod.rs`, `src/plugins/config_reloader.rs`.

## ADR-008: Single-Crate Module Layering for Kernel and Plugins

- Governs: REQ-022, REQ-025
- State: Accepted
- Context and constraints: Enforcing clean separation between the domain-agnostic Cordis kernel, shared service interfaces, vertical domain/transport plugins, and profile orchestration can be done either via workspace sub-crates or strict top-level modules inside `vietcalendar-rs`.
- Viable options: Option A — Single Cargo crate `vietcalendar-rs` partitioned into `src/kernel/`, `src/contracts/`, `src/plugins/`, and `src/profiles/`; vs. Option B — Multi-crate Cargo workspace (`crates/cordis-kernel`, `crates/vietcalendar-contracts`, `crates/vietcalendar-plugins`, `crates/vietcalendar-app`).
- Decision: Option A — Single Cargo crate with strict module boundaries (`src/kernel/` zero-domain invariant verified in unit tests and `RUST_STYLE_GUIDE.md`).
- Costs and risks: Keeps `Cargo.toml`, `Dockerfile` dependency layer caching, and single-binary distribution simple while allowing immediate migration to workspace sub-crates later if `kernel` is published standalone. Requires developer discipline (documented in `AGENTS.md`) to avoid importing `plugins` or `contracts` from `kernel`.
- Rejected alternatives: Option B complicates `Dockerfile` stub caching and CI build scripts for a single-application repository with no immediate external crate consumers.
- Affected components: `Cargo.toml`, `src/lib.rs`, `src/kernel/`, `src/contracts/`, `src/plugins/`, `src/profiles/`.

## Gate 1: normative review

| Check | Status | Evidence / required correction |
| --- | --- | --- |
| Codebase grounding | PASS | Grounded in `src/main.rs`, `Cargo.toml`, and `docs/RUST_STYLE_GUIDE.md` against DeepSeek Harness / Cordis architecture principles. |
| Attributable intent and explicit scope | PASS | `REQ-022`..`REQ-026` trace directly to user `/grill-me` decisions; WASM/FFI plugins and domain leakage in `kernel` excluded in `Non-goals`. |
| Observable requirements and meaningful acceptance criteria | PASS | `AC-022-1`..`AC-026-2` test wait-free lookup, out-of-order reactive mount, LIFO effect order, rollback on mount failure, patch reconciliation, and fail-safe YAML reload. |
| Assumptions and owner-deferred questions are explicit | PASS | Single-crate assumption recorded explicitly; zero `TODO` or `[PROVISIONAL]` items. |
| Architectural rationale respects requirements | PASS | `ADR-007` and `ADR-008` compare genuinely viable Rust plugin architectures and govern `REQ-022`..`REQ-026`. |
| IDs, references, and revision route are consistent | PASS | Fresh `REQ-022..REQ-026` and `ADR-007, ADR-008` registered cleanly in `docs/specs/INDEX.md`. |
