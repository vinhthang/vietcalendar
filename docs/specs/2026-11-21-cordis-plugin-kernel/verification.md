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

- Method: Integration test `test_live_yaml_hot_reload_and_malformed_rollback` in `tests/integration_test.rs` watching `TempDir`.
- Expected outcome: Valid YAML changes trigger debounced reload, updating live kernel plugins and `HolidayService` schedules without restarting.
- Execution: [Run 1](#run-1--2026-11-21)

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

## Real-world observations

No real-world confirmation has been recorded.
