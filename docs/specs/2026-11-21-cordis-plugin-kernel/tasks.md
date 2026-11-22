# Tasks: Cordis-Style Non-Blocking Plugin Kernel and Profiles

Sources: [spec.md](./spec.md), [design.md](./design.md)

## Task 1

**Implement zero-domain Cordis kernel core (`Context`, `Fiber`, `ServiceRegistry`, `EffectScope`, `reconcile`)**

- [x] Complete the work and its applicable checks.
- Covers: REQ-022, REQ-023, REQ-024, REQ-025 / AC-022-1, AC-022-2, AC-023-1, AC-023-2, AC-024-1, AC-024-2, AC-025-1, AC-025-2
- Files: `src/kernel/mod.rs`, `src/kernel/error.rs`, `src/kernel/effect.rs`, `src/kernel/event.rs`, `src/kernel/registry.rs`, `src/kernel/fiber.rs`, `src/kernel/context.rs`, `src/kernel/reconcile.rs`, `src/profiles/mod.rs`
- Dependencies: None
- Blocked by: None
- Work: Write failing unit tests for LIFO effect unwinding, wait-free `ArcSwap` service lookup, out-of-order reactive mount, cycle rejection, cascading unmount, failed-mount rollback, `ctx.spawn` task cancellation, and YAML patch reconciliation; implement `src/kernel/` and `src/profiles/mod.rs` with zero `.unwrap()` in non-test code.
- Completion evidence: All 6 kernel unit tests + zero-domain `src/kernel/` import check passed in [Run 1](./verification.md#run-1--2026-11-21).

## Task 2

**Implement YAML configs and `ConfigReloaderPlugin` (`notify` + `SIGHUP`)**

- [x] Complete the work and its applicable checks.
- Covers: REQ-025, REQ-026 / AC-025-1, AC-025-2, AC-026-1, AC-026-2
- Files: `config/cordis.yml`, `config/cordis.patch.yml`, `src/plugins/config_reloader.rs`
- Dependencies: Task 1
- Blocked by: None
- Work: Write default `cordis.yml` (`serve`, `mcp-stdio`, `http-only`) and `cordis.patch.yml`; write `tempfile`-backed hot-reload tests covering valid YAML updates and malformed YAML rollback; implement debounced bounded-channel `notify` + `SIGHUP` `ConfigReloaderPlugin`.
- Completion evidence: `test_profile_boots_and_clean_shutdown` and `test_live_yaml_hot_reload_and_malformed_rollback` passed in [Run 1](./verification.md#run-1--2026-11-21).

## Task 3

**Filter `notify` events in `ConfigReloaderPlugin` to ignore read-only access loops (Case 1 defect)**

- [x] Complete the work and its applicable checks.
- Covers: REQ-026 / AC-026-1
- Files: `src/plugins/config_reloader.rs`
- Dependencies: Task 2
- Blocked by: None
- Work: Add `test_hot_reload_does_not_self_trigger_on_read_events` (reproducing Linux `inotify` read-access reload loop) and `test_should_trigger_reload_filters_read_access_and_atime_events`; implement `should_trigger_reload` filtering out `EventKind::Access` (except `Close(Write)`), `AccessTime`, `Any`, and `Other`.
- Completion evidence: `plugins::config_reloader` (`5 passed; 0 failed`) and targeted `cargo mutants` (`2/2` caught = `100%`) verified in [Run 2](./verification.md#run-2--2026-11-22).

## Gate 2: traceability and readiness

| Requirement | AC | ADR or rationale | Design | Task | Planned verification | Status / reason |
| --- | --- | --- | --- | --- | --- | --- |
| REQ-022 | AC-022-1 | [ADR-007](./decisions.md#adr-007-native-compiled-in-rust-cordis-kernel-with-arcswap-registries-and-lifo-effectscope), [ADR-008](./decisions.md#adr-008-single-crate-module-layering-for-kernel-and-plugins) | [Components](./design.md#components) | [Task 1](#task-1) | [AC-022-1](./verification.md#ac-022-1-req-022) | PASS — lifecycle & zero-domain check mapped |
| REQ-022 | AC-022-2 | [ADR-007](./decisions.md#adr-007-native-compiled-in-rust-cordis-kernel-with-arcswap-registries-and-lifo-effectscope) | [Boundaries](./design.md#boundaries-and-recovery) | [Task 1](#task-1) | [AC-022-2](./verification.md#ac-022-2-req-022) | PASS — duplicate/missing fiber error check mapped |
| REQ-023 | AC-023-1 | [ADR-007](./decisions.md#adr-007-native-compiled-in-rust-cordis-kernel-with-arcswap-registries-and-lifo-effectscope) | [Interfaces](./design.md#interfaces-and-flow) | [Task 1](#task-1) | [AC-023-1](./verification.md#ac-023-1-req-023) | PASS — reactive out-of-order mount check mapped |
| REQ-023 | AC-023-2 | [ADR-007](./decisions.md#adr-007-native-compiled-in-rust-cordis-kernel-with-arcswap-registries-and-lifo-effectscope) | [Boundaries](./design.md#boundaries-and-recovery) | [Task 1](#task-1) | [AC-023-2](./verification.md#ac-023-2-req-023) | PASS — cycle & cascade teardown mapped |
| REQ-024 | AC-024-1 | [ADR-007](./decisions.md#adr-007-native-compiled-in-rust-cordis-kernel-with-arcswap-registries-and-lifo-effectscope) | [Components](./design.md#components) | [Task 1](#task-1) | [AC-024-1](./verification.md#ac-024-1-req-024) | PASS — LIFO disposer order test mapped |
| REQ-024 | AC-024-2 | [ADR-007](./decisions.md#adr-007-native-compiled-in-rust-cordis-kernel-with-arcswap-registries-and-lifo-effectscope) | [Boundaries](./design.md#boundaries-and-recovery) | [Task 1](#task-1) | [AC-024-2](./verification.md#ac-024-2-req-024) | PASS — failed-mount rollback test mapped |
| REQ-025 | AC-025-1 | [ADR-008](./decisions.md#adr-008-single-crate-module-layering-for-kernel-and-plugins) | [Interfaces](./design.md#interfaces-and-flow) | [Task 1](#task-1), [Task 2](#task-2) | [AC-025-1](./verification.md#ac-025-1-req-025) | PASS — YAML profile & patch merge mapped |
| REQ-025 | AC-025-2 | [ADR-008](./decisions.md#adr-008-single-crate-module-layering-for-kernel-and-plugins) | [Boundaries](./design.md#boundaries-and-recovery) | [Task 1](#task-1), [Task 2](#task-2) | [AC-025-2](./verification.md#ac-025-2-req-025) | PASS — unknown profile/plugin error mapped |
| REQ-026 | AC-026-1 | [ADR-007](./decisions.md#adr-007-native-compiled-in-rust-cordis-kernel-with-arcswap-registries-and-lifo-effectscope) | [Interfaces](./design.md#interfaces-and-flow) | [Task 2](#task-2), [Task 3](#task-3) | [AC-026-1](./verification.md#ac-026-1-req-026) | PASS — debounced YAML hot-reload & read-access filter mapped |
| REQ-026 | AC-026-2 | [ADR-007](./decisions.md#adr-007-native-compiled-in-rust-cordis-kernel-with-arcswap-registries-and-lifo-effectscope) | [Boundaries](./design.md#boundaries-and-recovery) | [Task 2](#task-2) | [AC-026-2](./verification.md#ac-026-2-req-026) | PASS — malformed YAML preservation mapped |

- Backward scope check: PASS — Tasks 1–3 own only `src/kernel/`, `src/profiles/`, `src/plugins/config_reloader.rs`, and `config/cordis*.yml`.
- Reference and catalog consistency: PASS — `REQ-022..REQ-026` and `ADR-007, ADR-008` match `docs/specs/INDEX.md` and resolve cleanly.
