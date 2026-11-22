# Decisions: HTTP REST API and OpenAPI Specification

Governing specification: [spec.md](./spec.md)

## Architectural rationale

- Governs: REQ-008, REQ-009, REQ-010, REQ-011, REQ-012

The HTTP REST layer (`src/handlers.rs` and `src/main.rs`) delegates all domain computation to [astronomical-calendar](../2026-11-21-astronomical-calendar/spec.md#req-003-bidirectional-solar-and-vietnamese-lunar-conversion) and [vietnam-holidays](../2026-11-21-vietnam-holidays/spec.md#req-005-weekend-and-fixed-solar-holiday-classification). `REQ-008` anchors `GET /` to Vietnam UTC+7 (`ADR-4`), `REQ-009` and `REQ-011` preserve legacy Vert.x query-parameter compatibility (`dd`, `mm`/`MM`, `yyyy`, `timeZone`/`timezone`), and `REQ-012` applies `CorsLayer` (`Any`) plus Utoipa `SwaggerUi` alongside the two core architectural choices captured in `ADR-003` and `ADR-004`.

## ADR-003: Symmetric ISO-8601 `/convert/.../{date}` Routes Alongside Legacy `/lunar` Query Route

- Governs: REQ-009, REQ-010
- State: Accepted
- Context and constraints: The legacy Java Vert.x service exposed `/lunar/{ddMMyyyy}`, which created `DD-MM` vs `MM-DD` ambiguity, lacked reverse lunar-to-solar conversion, and could not express `leap=true` cleanly, while existing clients still depended on `GET /lunar?dd=...&mm=...&yyyy=...`.
- Viable options: Option A — Introduce symmetric ISO-8601 path endpoints `/convert/solar-to-lunar/{date}` and `/convert/lunar-to-solar/{date}` with `?leap=` and `?timezone=` while retaining `GET /lunar` query parameters; vs. Option B — Keep `/lunar/{ddMMyyyy}` and add ad-hoc suffix flags for leap months.
- Decision: Option A — Symmetric `/convert/solar-to-lunar/{date}` and `/convert/lunar-to-solar/{date}` routes alongside `/lunar`.
- Costs and risks: Eliminates day/month order ambiguity and cleanly supports optional `?leap=true/false`, at the cost of maintaining two solar-to-lunar endpoints (`/lunar` and `/convert/solar-to-lunar/{date}`).
- Rejected alternatives: Option B perpetuates `DD-MM` vs `MM-DD` ambiguity and forces non-standard path encoding for leap months.
- Affected components: `src/handlers.rs` (`get_lunar`, `get_solar_to_lunar`, `get_lunar_to_solar`), `src/main.rs` (`Router`, `ApiDoc`).

## ADR-004: Centralized `AppError` JSON Mapping Over Handler `String` Errors

- Governs: REQ-009, REQ-010, REQ-011, REQ-012
- State: Accepted
- Context and constraints: Axum serializes `Result<Json<T>, String>` with `HTTP 200 OK` if `String` implements `IntoResponse` only via plain string body semantics or fails type constraints, breaking REST API client error detection.
- Viable options: Option A — Define `AppError { BadRequest(String), NotFound(String) }` implementing `IntoResponse` to emit `StatusCode::BAD_REQUEST`/`NOT_FOUND` with JSON `ErrorResponse { error }`; vs. Option B — Construct `(StatusCode, Json<ErrorResponse>)` tuples manually in every handler branch.
- Decision: Option A — Centralized `AppError` implementing `IntoResponse`.
- Costs and risks: Guarantees uniform JSON `{ "error": "..." }` payloads and HTTP 4xx status codes with concise `?` / `.ok_or_else(...)` handler ergonomics. Minor caveat (`TD-06`): Axum built-in `Query<T>` extraction failures bypass `AppError` and emit default plain-text `400`.
- Rejected alternatives: Option B duplicates response-tuple boilerplate across every handler error branch.
- Affected components: `src/handlers.rs` (`AppError`, `ErrorResponse`), `src/main.rs` (`ApiDoc`).

## Gate 1: normative review

| Check | Status | Evidence / required correction |
| --- | --- | --- |
| Codebase grounding | PASS | Grounded in `src/handlers.rs`, `src/main.rs`, `src/models.rs`, and `docs/ARCHITECTURE_DECISIONS.md` (ADR-3, ADR-4, ADR-7, ADR-11). |
| Attributable intent and explicit scope | PASS | `REQ-008`..`REQ-012` map to observed REST handlers and ADRs; MCP endpoints and `TD-06` code changes excluded in `Non-goals`. |
| Observable requirements and meaningful acceptance criteria | PASS | `AC-008-1`..`AC-012-2` specify HTTP paths, payloads, UTC+7 behavior, leap queries, and `AppError` 400/404 status mapping. |
| Assumptions and owner-deferred questions are explicit | PASS | `TD-06` OpenAPI/extractor caveats explicitly documented; zero `TODO` or `[PROVISIONAL]` items. |
| Architectural rationale respects requirements | PASS | `ADR-003`, `ADR-004`, and `## Architectural rationale` govern `REQ-008`..`REQ-012`. |
| IDs, references, and revision route are consistent | PASS | `REQ-008..REQ-012` and `ADR-003, ADR-004` match `docs/specs/INDEX.md`; cross-spec links valid. |
