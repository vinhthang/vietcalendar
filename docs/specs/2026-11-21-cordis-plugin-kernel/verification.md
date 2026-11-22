# Verification: Cordis-Style Non-Blocking Plugin Kernel and Profiles

Governing specification: [spec.md](./spec.md)

## Planned checks

### AC-022-1 (REQ-022)

- Method: Unit tests `kernel::tests::test_lifo_effect_disposal_and_task_cancellation`, `test_child_fiber_fork_and_lifo_parent_teardown` + `rg '^\s*use .*(contracts|plugins|axum|chrono)' src/kernel/`.
- Expected outcome: All spawned tasks terminate upon fiber disposal/shutdown and `src/kernel/` compiles with zero domain/transport imports.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-022-2 (REQ-022)

- Method: Unit test `kernel::tests::test_duplicate_and_missing_fiber_errors` invoking duplicate `mount` and unknown `dispose`.
- Expected outcome: Returns `Err(KernelError)` cleanly without panicking or affecting sibling fibers.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-023-1 (REQ-023)

- Method: Unit test `kernel::tests::test_reactive_out_of_order_mount_and_cascade_suspend` + `kernel_wait_free_inject` benchmark.
- Expected outcome: `Consumer` mounts automatically once `Provider` provides `S`, and `inject` reads lock-free via `ArcSwap`.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-023-2 (REQ-023)

- Method: Unit tests `kernel::tests::test_cycle_detection_and_failed_mount_rollback` and `test_reactive_out_of_order_mount_and_cascade_suspend`.
- Expected outcome: Cycle returns `KernelError::CycleDetected`; provider disposal tears down `Consumer` cleanly before completing.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-024-1 (REQ-024)

- Method: Unit test `kernel::tests::test_lifo_effect_disposal_and_task_cancellation` registering `[E1, E2, E3]` disposers.
- Expected outcome: Recorded unwind trace equals `["E3", "E2", "E1"]`.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-024-2 (REQ-024)

- Method: Unit test `kernel::tests::test_cycle_detection_and_failed_mount_rollback` mounting a failing plugin after providing a service.
- Expected outcome: Partial service is removed, registered effect executes during rollback, and registry stays clean.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-025-1 (REQ-025)

- Method: Unit test `kernel::tests::test_profile_patch_and_reconcile` and integration test `test_profile_boots_and_clean_shutdown`.
- Expected outcome: Enabled/disabled plugins and deep-merged config changes mount, reconfigure, and dispose deterministically.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-025-2 (REQ-025)

- Method: Unit inspection and negative error paths in `resolve_profile` / `Kernel::reconcile` (`KernelError::UnknownProfile`, `UnknownPlugin`).
- Expected outcome: Returns `Err(KernelError)` while leaving existing active fibers undisturbed.
- Execution: [Run 1](#run-1--2026-11-21)

### AC-026-1 (REQ-026)

- Method: Integration test `test_live_yaml_hot_reload_and_malformed_rollback` in `tests/integration_test.rs` watching `TempDir` plus unit tests `plugins::config_reloader::tests::{test_hot_reload_does_not_self_trigger_on_read_events, test_should_trigger_reload_filters_read_access_and_atime_events}`.
- Expected outcome: Valid YAML changes trigger debounced reload, updating live kernel plugins and `HolidayService` schedules without restarting, and read-only `inotify` access events never self-trigger reload loops.
- Execution: [Run 1](#run-1--2026-11-21), [Run 2](#run-2--2026-11-22)

### AC-026-2 (REQ-026)

- Method: Integration test `test_live_yaml_hot_reload_and_malformed_rollback` writing malformed YAML after active boot.
- Expected outcome: Reloader ignores bad YAML cleanly and preserves the previous valid configuration and holiday schedules.
- Execution: [Run 1](#run-1--2026-11-21)

## Execution evidence

### Run 1 — 2026-11-21

- Checked: REQ-022, REQ-023, REQ-024, REQ-025, REQ-026 / AC-022-1..AC-026-2 (`rustc 1.92.0` / `cargo 1.92.0`)
- Command: `RUSTFLAGS="-D warnings" rtk proxy cargo test --offline --all-targets` (exit code: `0`), `rtk rg '^\s*use .*(contracts|plugins|axum|chrono)' src/kernel/` (exit code: `1` / zero matches), and `rtk proxy cargo bench --offline --bench calendar_bench -- --warm-up-time 1 --measurement-time 2` (exit code: `0`)
- Output excerpt:
  ```text
  running 23 tests
  test kernel::tests::test_duplicate_and_missing_fiber_errors ... ok
  test kernel::tests::test_cycle_detection_and_failed_mount_rollback ... ok
  test kernel::tests::test_child_fiber_fork_and_lifo_parent_teardown ... ok
  test kernel::tests::test_profile_patch_and_reconcile ... ok
  test kernel::tests::test_lifo_effect_disposal_and_task_cancellation ... ok
  test kernel::tests::test_reactive_out_of_order_mount_and_cascade_suspend ... ok
  ...
  test result: ok. 23 passed; 0 failed; 0 ignored

  Running tests/integration_test.rs
  test test_profile_boots_and_clean_shutdown ... ok
  test test_live_yaml_hot_reload_and_malformed_rollback ... ok
  test result: ok. 6 passed; 0 failed; 0 ignored

  kernel_wait_free_inject time: [58.612 ns 58.816 ns 59.155 ns]
  kernel_use_exit=1 (0 domain imports in src/kernel/)
  ```
- Result: AC-022-1 `PASS`, AC-022-2 `PASS`, AC-023-1 `PASS`, AC-023-2 `PASS`, AC-024-1 `PASS`, AC-024-2 `PASS`, AC-025-1 `PASS`, AC-025-2 `PASS`, AC-026-1 `PASS`, AC-026-2 `PASS` — all Cordis kernel lifecycle, reactive injection (`58.8 ns`), `ctx.fork`, YAML reconciliation, and `notify` hot-reload checks passed with 0 compiler warnings.

### Run 2 — 2026-11-22

- Checked: REQ-026 / AC-026-1, AC-026-2 Case 1 `notify` read-event loop fix (`rustc 1.97.1` / `cargo 1.97.1`, `rustfmt 1.9.0`, `clippy 0.1.97`)
- Command: `cargo fmt --all -- --check && cargo clippy --offline --all-targets -- -D warnings && RUSTFLAGS="-D warnings" cargo test --offline --all-targets && CARGO_NET_OFFLINE=true cargo mutants --timeout 60 -j 2 -f src/plugins/config_reloader.rs --re should_trigger_reload -- --lib plugins::config_reloader` (exit code: `0`)
- Output excerpt:
  ```text
  RED baseline (test_hot_reload_does_not_self_trigger_on_read_events before filter):
    test result: FAILED. 0 passed; 1 failed

  GREEN verification after should_trigger_reload filter:
    cargo clippy: No issues found
    plugins::config_reloader: ok. 5 passed; 0 failed
    src/lib.rs unit tests: ok. 72 passed; 0 failed; 0 ignored
    tests/integration_test.rs: ok. 8 passed; 0 failed; 0 ignored
    cargo mutants (should_trigger_reload): 2 mutants tested: 2 caught (100%)
  ```
- Result: AC-026-1 `PASS`, AC-026-2 `PASS` — `should_trigger_reload` eliminates Linux `inotify` read-access self-triggers while preserving live config/holiday hot-reload and malformed YAML rollback.

## Real-world observations

- `2026-11-22`: Live production pod `apps/vietcalendar-68f7b7d6f5-76r5w` (`v0.2.0` at `sha256:2f42b796...` on `amd11` Linux) exhibited continuous `INFO hot-reloaded configuration and holiday schedules` log lines (`~1/s`) caused by Linux `inotify` `EventKind::Access` events on YAML reads; reproduced via `test_hot_reload_does_not_self_trigger_on_read_events` and fixed in Case 1 [Task 3](./tasks.md#task-3) (awaiting live cluster re-verification after image rollout before promoting to `Verified`).
