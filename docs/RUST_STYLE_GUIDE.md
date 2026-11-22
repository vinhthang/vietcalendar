# Rust Coding Style Guide — VietCalendar (`vietcalendar-rs`)

This guide defines the **Pragmatic Idiomatic Rust** coding standards for all new and modified Rust source files (`src/**/*.rs`, `tests/**/*.rs`) in `vietcalendar-rs`. It extends the official [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/), default `rustfmt`, and `clippy::all` (`-D warnings`).

---

## 1. Scope & Grandfathering Policy

- **Applies to:** All newly created or modified Rust code in `src/` and `tests/`.
- **Grandfathered baseline tech debt:** Pre-existing migration artifacts from the Java 11 / Vert.x port are explicitly documented in [`docs/PROJECT_ANALYSIS.md`](PROJECT_ANALYSIS.md#4-catalog-of-discovered-quirks-bugs--technical-debt) (`TD-01`–`TD-08`). Do **not** mix sweeping style cleanups into unrelated feature or bug-fix PRs; remediate grandfathered items in focused, spec-backed cleanup PRs:
  - JavaDoc blocks (`/** @author ... @param ... @return ... */`), commented-out `CC2K` lines, `pub fn int()`, and comment typos in `src/calendar.rs`.
  - Unused `#[allow(non_snake_case)]` attributes in `src/handlers.rs`.
  - Misnamed `is_solar_leap` in `src/services.rs`.
  - Legacy `.unwrap()` calls on `HOLIDAY_CACHE` `RwLock` (`src/services.rs`), `TcpListener::bind` / `axum::serve` (`src/main.rs`), and `.unwrap_or_default()` JSON serialization in `src/mcp.rs`.
  - Sentinel `(0, 0, 0)` return and `lunar_leap: i32` parameter in `src/calendar.rs::convert_lunar_to_solar`.
- **CI alignment:** Keep formatting compatible with default `rustfmt` (do not add overriding `rustfmt.toml` or workspace `[lints]` tables unless existing files are cleaned up simultaneously so `cargo fmt --all -- --check` and `cargo clippy -- -D warnings` remain green).

---

## 2. Formatting, Imports & Linting

- **Formatter:** Run `cargo fmt --all -- --check` before every commit. All code must adhere to default `rustfmt` (4-space indentation, 100-character max width, trailing commas on multi-line blocks).
- **Linter:** Code must compile cleanly under `cargo clippy -- -D warnings` (default `clippy::all` lint set). Never suppress Clippy warnings with `#[allow(...)]` unless a documented, unavoidable reason exists.
- **Import grouping:** Group `use` statements in three sections separated by a blank line:
  1. `std` imports (`use std::...;`)
  2. External crate imports (`use axum::...;`, `use chrono::...;`, `use serde::...;`)
  3. Crate-local / library imports (`use crate::...;` or `use vietcalendar_rs::...;`)
- **Wildcard imports:** Forbidden in production modules (`src/`). Permitted only inside `#[cfg(test)] mod tests { use super::*; }`.

---

## 3. Naming & Module Boundaries

- **Casing:** Follow standard Rust casing strictly:
  - `snake_case` for modules, functions, methods, parameters, local variables, and struct fields.
  - `UpperCamelCase` for structs, enums, enum variants, and traits.
  - `SCREAMING_SNAKE_CASE` for `const` and `static` items.
- **Serde & query aliases:** Never add `#[allow(non_snake_case)]` when Rust identifiers are already `snake_case` (only use `#[serde(alias = "...")]` or `#[serde(rename = "...")]` for external wire casing such as `MM` or `timeZone`).
- **Accurate domain naming:** Name functions and types for what they actually compute. Never prefix a lunar/Metonic calculation with `is_solar_*` or reuse ambiguous helper names like `int()`.
- **Layered module boundaries (`0.2.0` Cordis architecture):**
  - `src/kernel/`: Zero-domain Cordis kernel (`Context`, `Fiber`, wait-free `ArcSwap` `ServiceRegistry`, LIFO `EffectScope`, `EventBus`, YAML `reconcile`). Must never import `contracts`, `plugins`, `profiles`, `axum`, `chrono`, or `utoipa`.
  - `src/contracts/`: Abstract Service Definition traits (`CalendarService`, `HolidayService`, `HttpRouterService`, `McpRegistryService`) and shared DTOs (`DateMonthYear`, `LunarDate`, `YearHolidaySchedule`, `ErrorResponse`).
  - `src/plugins/`: Decoupled vertical domain and transport plugins (`calendar_vn`, `holidays_vn`, `http_server`, `http_calendar`, `http_holidays`, `openapi`, `mcp_core`, `mcp_tools`, `mcp_stdio`, `mcp_http`, `config_reloader`, `telemetry`).
  - `src/profiles/`: Declarative YAML loader (`config/cordis.yml`, `config/cordis.patch.yml`) and `PluginFactoryRegistry`.
  - `src/main.rs` & `src/bin/vietcalendar-mcp.rs`: CLI argument parsing, OS shutdown signals, and profile bootstrap.
- **Zero business-logic duplication:** Transport plugins (`http_calendar`, `http_holidays`, `mcp_tools`) must **never** duplicate domain rules (such as holiday or calendar math); delegate strictly through `CalendarService` and `HolidayService`.

---

## 4. Domain Modeling & Type Safety

- **Dedicated lunar types:** **NEVER** store or return lunar dates in `chrono::NaiveDate` (lunar months may have 30 days in month 2, which panics `NaiveDate::from_ymd_opt`). Always use `models::LunarDate` (`{ day: i32, month: i32, year: i32, is_leap: bool }`).
- **Explicit types over magic values:** In new or refactored APIs, use `bool`, `enum`, `Option<T>`, and `Result<T, E>` instead of `0`/`1` integers (`lunar_leap: i32`) or sentinel tuples (`(0, 0, 0)`).
- **Boundary validation:** Validate all external inputs at transport/service boundaries before running calendar math:
  - Solar dates via `NaiveDate::from_ymd_opt` / `NaiveDate::parse_from_str`.
  - Lunar months in `1..=12` and lunar days against the actual month length (`29` or `30` days—do not allow day `30` on a 29-day *tháng thiếu* to roll over silently into the next month in new/fixed endpoints).
  - Finite numeric timezone offsets within realistic bounds (e.g., `-12.0..=14.0` or explicit `is_finite()`).
- **Checked integer conversions:** Prefer `u32::try_from(v)` / `i32::try_from(v)` over unchecked `as` casts (`i64 as u32`, `i64 as i32`) when parsing JSON or query integers.
- **Standard trait derives:** Derive `Debug, Clone, PartialEq, Eq` on domain/DTO structs and `Debug` on error enums; add `Serialize`, `Deserialize`, and `ToSchema` where serialized or documented.

---

## 5. Error Handling & Zero-Panic Policy

- **No new production `.unwrap()` / `.expect()`:** Never introduce `.unwrap()`, `.expect()`, `panic!()`, or unchecked slice indexing (`parts[0]`) on runtime paths in `src/` outside `#[cfg(test)]`.
  - Propagate errors with `?`, `Result`, `Option`, `.get(...)`, and `.ok_or_else(...)`.
  - Handle `std::sync::RwLock` / `Mutex` poisoning gracefully (`.map_err(...)` or poisoned-guard recovery with logging) rather than `.unwrap()`.
  - Report startup/bind errors cleanly from `main()` / `run_http_server` using `Result` propagation or structured `tracing::error!`.
- **HTTP errors (`src/handlers.rs`):** Handlers must return `Result<Json<T>, AppError>` (or `impl IntoResponse`), **never** `Result<Json<T>, String>` (which Axum serializes with `HTTP 200 OK`). Validation errors map to `AppError::BadRequest` (`HTTP 400`) with JSON `ErrorResponse` (`{ "error": "<message>" }`).
- **MCP JSON-RPC errors (`src/mcp.rs`):** Return standard JSON-RPC error codes: `-32700` (Parse error), `-32601` (Method/tool not found), `-32602` (Invalid params), and `-32603` (Internal error). Never silently swallow serialization errors with `.unwrap_or_default()`.
- **Test exception:** `.unwrap()` and `.expect("clear failure context")` are permitted inside `#[cfg(test)] mod tests` and `tests/`.

---

## 6. Async Runtime, Concurrency & Caching

- **Never hold `std::sync` locks across `.await`:** Extract/clone data and drop `std::sync::RwLock` / `Mutex` guards immediately before any `.await` or non-trivial work, or use `tokio::sync::RwLock` for async-held state.
- **Complete cache keys:** Any memoized/cached computation must key on **all inputs that influence the result** (e.g., `(solar_year, time_zone_quantized)` rather than `year` alone) so distinct requests cannot poison shared cache entries.
- **Bounded streaming & session hygiene:** Prefer bounded channels (`tokio::sync::mpsc::channel`) with explicit backpressure handling or session limits over `mpsc::unbounded_channel` in new streaming transports, and validate `sessionId` for all session-bound messages including notifications.
- **Graceful shutdown & telemetry:** New server lifecycle changes should handle OS shutdown signals (`tokio::signal`) and shut down/flush the OpenTelemetry `TracerProvider` cleanly without stalling startup when no OTLP collector is configured.

---

## 7. Documentation (`rustdoc`) & Comments

- **Public item docs:** Every new or modified `pub` module, struct, enum, trait, function, and field must have a `///` doc comment (or `//!` at module top) following Rustdoc conventions:
  - Include `# Errors` on functions returning `Result` and `# Panics` if any invariant can panic.
  - Include concise `# Examples` on core domain helpers when helpful.
- **Idiomatic comments:** Never introduce JavaDoc (`/** @param */`, `@author`, `@return`) or commented-out legacy code. Write comments to explain **why** (astronomical conventions, Vietnam Labor Code rules, non-obvious invariants), not **what** obvious code does.

---

## 8. Contract Synchronization (OpenAPI, MCP & Docs)

- **OpenAPI (`utoipa`):** Whenever HTTP endpoints, parameters, query aliases, or response structs change, update `#[utoipa::path(...)]`, `#[derive(IntoParams, ToSchema)]`, and `ApiDoc` in `src/main.rs` so `/api-docs/openapi.json` remains accurate.
- **MCP schemas:** Keep `tools/list` JSON schemas and `resources/list` entries in `src/mcp.rs` strictly aligned with actual `tools/call` and `resources/read` parameter handling and output fields.
- **Repo docs & specs:** Synchronize `README.md`, `docs/ARCHITECTURE_DECISIONS.md`, and `docs/specs/` packages for any behavior or breaking API change per [`AGENTS.md`](../AGENTS.md).

---

## 9. Testing Conventions

- **Unit tests:** Keep focused unit tests inside `#[cfg(test)] mod tests` in each module. Use `#[tokio::test]` for async handlers and session state.
- **Integration tests:** Place cross-module and end-to-end HTTP/Axum/MCP tests in `tests/integration_test.rs` (including router status codes, error payloads, and OpenAPI/MCP flows for new routes).
- **Required coverage:** Every behavior change must test both happy paths and boundary/failure conditions (e.g., leap vs non-leap months, 29-day vs 30-day lunar months, solar year-boundary Tết Eve, weekend compensatory chains, invalid dates/JSON-RPC).
- **No test bypasses:** Never add `#[ignore]` to hide failing CI tests or write vacuous assertions.

---

## 10. Pre-PR Verification Checklist

Before requesting review or merging any Rust change:

- [ ] `cargo fmt --all -- --check` exits `0`.
- [ ] `cargo clippy -- -D warnings` exits `0` with no new `#[allow(...)]` suppressions.
- [ ] `cargo test --verbose` passes completely (all unit and integration tests).
- [ ] Zero new `.unwrap()`, `.expect()`, or unchecked slice indexing in `src/` production paths.
- [ ] All new or modified `pub` items have `///` rustdoc comments (no JavaDoc syntax).
- [ ] OpenAPI (`ApiDoc`), MCP schemas, `README.md`, `docs/ARCHITECTURE_DECISIONS.md`, and `docs/specs/` match the implementation.
