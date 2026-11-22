use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use chrono::{Datelike, NaiveDate, Utc};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tempfile::TempDir;
use tokio_stream::StreamExt as _;
use tower::ServiceExt;

use vietcalendar_rs::contracts::calendar::CalendarService;
use vietcalendar_rs::contracts::holidays::HolidayService;
use vietcalendar_rs::contracts::http::HttpRouterService;
use vietcalendar_rs::contracts::mcp::McpRegistryService;
use vietcalendar_rs::contracts::models::{DateMonthYear, ErrorResponse};
use vietcalendar_rs::kernel::fiber::FiberState;
use vietcalendar_rs::plugins::config_reloader::{reload_from_disk, RuntimeConfigAnchor};
use vietcalendar_rs::plugins::mcp_http::McpSessionManager;
use vietcalendar_rs::profiles::{boot_profile, build_default_factories};

async fn body_bytes(body: Body) -> Vec<u8> {
    body.collect().await.unwrap().to_bytes().to_vec()
}

async fn body_json(body: Body) -> Value {
    let bytes = body_bytes(body).await;
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn test_profile_boots_and_clean_shutdown() {
    for profile in ["serve", "http-only", "mcp-stdio"] {
        let kernel = boot_profile(Path::new("config"), profile, None, Some(false))
            .await
            .unwrap();
        assert!(kernel.inject::<dyn CalendarService>().is_some());
        assert!(kernel.inject::<dyn HolidayService>().is_some());
        if profile != "mcp-stdio" {
            assert!(kernel.inject::<dyn HttpRouterService>().is_some());
        } else {
            assert!(kernel.inject::<dyn McpRegistryService>().is_some());
        }
        kernel.shutdown().await;
        assert!(kernel.fiber_states().is_empty());
    }
}

#[tokio::test]
async fn test_http_rest_endpoints_end_to_end() {
    let kernel = boot_profile(Path::new("config"), "serve", None, Some(false))
        .await
        .unwrap();
    let router_svc = kernel.inject::<dyn HttpRouterService>().unwrap();
    let cal_svc = kernel.inject::<dyn CalendarService>().unwrap();
    let router = router_svc.current_router();

    // 1. GET /
    let resp = router
        .clone()
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .and_then(|v| v.to_str().ok()),
        Some("*")
    );
    let today_dmy: DateMonthYear =
        serde_json::from_value(body_json(resp.into_body()).await).unwrap();
    assert!(today_dmy.yyyy >= 2024);

    // 2. GET /lunar (uppercase MM and timeZone aliases)
    let resp = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/lunar?dd=12&MM=9&yyyy=2015&timeZone=7.0")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let lunar: DateMonthYear = serde_json::from_value(body_json(resp.into_body()).await).unwrap();
    assert_eq!(
        lunar,
        DateMonthYear {
            dd: 30,
            mm: 7,
            yyyy: 2015,
            leap: None
        }
    );

    // 3. Query deserialization error -> JSON ErrorResponse 400 (TD-06 fixed)
    let resp = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/lunar?dd=notanumber&mm=9&yyyy=2015")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let err_payload: ErrorResponse =
        serde_json::from_value(body_json(resp.into_body()).await).unwrap();
    assert!(!err_payload.error.is_empty());

    // 4. GET /convert/solar-to-lunar/{date}
    let resp = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/convert/solar-to-lunar/2015-09-12")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let s2l: DateMonthYear = serde_json::from_value(body_json(resp.into_body()).await).unwrap();
    assert_eq!((s2l.dd, s2l.mm, s2l.yyyy), (30, 7, 2015));

    // 5. GET /convert/lunar-to-solar/{date} (regular + leap)
    let resp = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/convert/lunar-to-solar/2004-02-11?leap=true")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let l2s: DateMonthYear = serde_json::from_value(body_json(resp.into_body()).await).unwrap();
    assert_eq!((l2s.dd, l2s.mm, l2s.yyyy), (31, 3, 2004));

    // 6. Strict day-30 rejection on a 29-day month (TD-03 fixed)
    let m29 = (1..=12)
        .find(|&m| cal_svc.lunar_month_days(m, 2024, false, 7.0).unwrap() == 29)
        .unwrap();
    let resp = router
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/convert/lunar-to-solar/2024-{m29:02}-30"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let day30_err: ErrorResponse =
        serde_json::from_value(body_json(resp.into_body()).await).unwrap();
    assert!(day30_err.error.contains("29 days"));

    for bad_uri in [
        "/convert/lunar-to-solar/2147483647-11-01",
        "/convert/lunar-to-solar/10000000-01-01",
    ] {
        let resp = router
            .clone()
            .oneshot(Request::builder().uri(bad_uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::BAD_REQUEST,
            "expected 400 on {bad_uri}"
        );
        let err_body: ErrorResponse =
            serde_json::from_value(body_json(resp.into_body()).await).unwrap();
        assert!(!err_body.error.is_empty());
    }

    // 7. GET /vietnam-holiday: 2024-2026 schedules, working weekends, unconfigured fallback, timezone
    let cases = [
        ("/vietnam-holiday?dd=29&mm=4&yyyy=2024", true), // 2024 bridge day
        ("/vietnam-holiday?dd=4&mm=5&yyyy=2024", false), // 2024 working Saturday
        ("/vietnam-holiday?dd=26&mm=4&yyyy=2025", false), // 2025 working Saturday
        ("/vietnam-holiday?dd=27&mm=4&yyyy=2026", true), // 2026 Hung Kings compensatory Mon
        ("/vietnam-holiday?dd=22&mm=8&yyyy=2026", false), // 2026 working Saturday
        ("/vietnam-holiday?dd=1&mm=1&yyyy=2099", false), // Unconfigured Thursday -> false
        ("/vietnam-holiday?dd=3&mm=1&yyyy=2099", true),  // Unconfigured Saturday -> true
    ];
    for (uri, expected) in cases {
        let resp = router
            .clone()
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK, "failed status on {uri}");
        let val: bool = serde_json::from_value(body_json(resp.into_body()).await).unwrap();
        assert_eq!(val, expected, "unexpected holiday value for {uri}");
    }

    let resp_tz_err = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/vietnam-holiday?dd=1&mm=1&yyyy=2026&timezone=20.0")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_tz_err.status(), StatusCode::BAD_REQUEST);

    // 8. OpenAPI JSON omits unused LunarDate schema and includes all 5 paths
    let resp = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api-docs/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let openapi = body_json(resp.into_body()).await;
    assert!(openapi["paths"]["/vietnam-holiday"].is_object());
    assert!(openapi["paths"]["/convert/solar-to-lunar/{date}"].is_object());
    assert!(openapi["components"]["schemas"]["DateMonthYear"].is_object());
    assert!(openapi["components"]["schemas"]["ErrorResponse"].is_object());
    assert!(openapi["components"]["schemas"]["LunarDate"].is_null());

    // 9. Swagger UI endpoint responds without error
    let resp = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/swagger-ui/")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(
        resp.status() == StatusCode::OK || resp.status().is_redirection(),
        "unexpected swagger status: {}",
        resp.status()
    );

    kernel.shutdown().await;
}

#[tokio::test]
async fn test_live_plugin_unmount_updates_router_openapi_and_mcp() {
    let kernel = boot_profile(Path::new("config"), "serve", None, Some(false))
        .await
        .unwrap();
    let router_svc = kernel.inject::<dyn HttpRouterService>().unwrap();
    let mcp_svc = kernel.inject::<dyn McpRegistryService>().unwrap();

    // Dispose http_holidays live -> /vietnam-holiday immediately returns 404 & disappears from OpenAPI!
    kernel.dispose("http_holidays").await.unwrap();
    let resp = router_svc
        .current_router()
        .oneshot(
            Request::builder()
                .uri("/vietnam-holiday?dd=1&mm=1&yyyy=2026")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    let openapi_resp = router_svc
        .current_router()
        .oneshot(
            Request::builder()
                .uri("/api-docs/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let openapi = body_json(openapi_resp.into_body()).await;
    assert!(openapi["paths"]["/vietnam-holiday"].is_null());

    // Dispose mcp_tools live -> its 5 tools vanish while the 11 niche tools remain intact!
    kernel.dispose("mcp_tools").await.unwrap();
    let list_resp = mcp_svc
        .dispatch(vietcalendar_rs::contracts::mcp::JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "tools/list".into(),
            params: None,
        })
        .unwrap();
    let tools = list_resp.result.unwrap()["tools"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(tools.len(), 11);
    assert!(!tools.iter().any(|t| t["name"] == "get_today_lunar"));

    kernel.shutdown().await;
}

#[tokio::test]
async fn test_mcp_http_direct_post_tools_and_resources() {
    let kernel = boot_profile(Path::new("config"), "serve", None, Some(false))
        .await
        .unwrap();
    let router_svc = kernel.inject::<dyn HttpRouterService>().unwrap();
    let cal_svc = kernel.inject::<dyn CalendarService>().unwrap();
    let router = router_svc.current_router();

    // 1. Direct POST /mcp -> initialize
    let resp = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/mcp")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let init_json = body_json(resp.into_body()).await;
    assert_eq!(init_json["result"]["protocolVersion"], "2024-11-05");

    // 2. Notification on POST /mcp/sse -> 204 No Content
    let resp_notif = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/mcp/sse")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_notif.status(), StatusCode::NO_CONTENT);

    // 3. get_year_holidays(2026) includes compensatory, bridge, and working_weekends (TD-01 fixed)
    let resp_2026 = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/mcp")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"get_year_holidays","arguments":{"year":2026}}}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_2026.status(), StatusCode::OK);
    let rpc_2026 = body_json(resp_2026.into_body()).await;
    let inner_text = rpc_2026["result"]["content"][0]["text"].as_str().unwrap();
    let parsed_2026: Value = serde_json::from_str(inner_text).unwrap();
    assert_eq!(parsed_2026["configured"], true);
    assert_eq!(parsed_2026["total_holidays"], 13);
    assert_eq!(parsed_2026["working_weekends"][0], "2026-08-22");

    // 4. convert_lunar_to_solar strict day-30 rejection on 29-day month (TD-03 fixed)
    let m29 = (1..=12)
        .find(|&m| cal_svc.lunar_month_days(m, 2024, false, 7.0).unwrap() == 29)
        .unwrap();
    let resp_bad_day = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/mcp")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{{"name":"convert_lunar_to_solar","arguments":{{"day":30,"month":{m29},"year":2024}}}}}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    let bad_day_json = body_json(resp_bad_day.into_body()).await;
    assert_eq!(bad_day_json["error"]["code"], -32602);

    // 5. resources/list & resources/templates/list
    let resp_res_list = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/mcp")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"jsonrpc":"2.0","id":4,"method":"resources/list"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let res_list = body_json(resp_res_list.into_body()).await;
    let cur_year = (Utc::now() + chrono::Duration::hours(7))
        .date_naive()
        .year();
    let expected_uri = format!("calendar://holidays/{cur_year}");
    let uris: Vec<&str> = res_list["result"]["resources"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|r| r["uri"].as_str())
        .collect();
    assert!(uris.contains(&"calendar://today"));
    assert!(uris.contains(&expected_uri.as_str()));

    let resp_tmpl = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/mcp")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"jsonrpc":"2.0","id":5,"method":"resources/templates/list"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let tmpl_json = body_json(resp_tmpl.into_body()).await;
    let templates: Vec<&str> = tmpl_json["result"]["resourceTemplates"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|t| t["uriTemplate"].as_str())
        .collect();
    assert_eq!(templates.len(), 3);
    assert!(templates.contains(&"calendar://holidays/{year}"));
    assert!(templates.contains(&"canchi://solar-terms/{year}"));
    assert!(templates.contains(&"retail://calendar/{year}"));

    kernel.shutdown().await;
}

#[tokio::test]
async fn test_mcp_sse_session_notification_404_and_backpressure_503() {
    let temp = TempDir::new().unwrap();
    fs::write(
        temp.path().join("cordis.yml"),
        r#"
profiles:
  serve:
    - id: calendar_vn
      plugin: calendar_vn
      enabled: true
      config: {}
    - id: holidays_vn
      plugin: holidays_vn
      enabled: true
      config: {}
    - id: http_server
      plugin: http_server
      enabled: true
      config:
        bind: false
    - id: mcp_core
      plugin: mcp_core
      enabled: true
      config: {}
    - id: mcp_tools
      plugin: mcp_tools
      enabled: true
      config: {}
    - id: mcp_http
      plugin: mcp_http
      enabled: true
      config:
        session_channel_capacity: 1
"#,
    )
    .unwrap();

    let kernel = boot_profile(temp.path(), "serve", None, Some(false))
        .await
        .unwrap();
    let router_svc = kernel.inject::<dyn HttpRouterService>().unwrap();
    let sessions = kernel.inject::<McpSessionManager>().unwrap();
    let router = router_svc.current_router();

    // 1. Notification to unknown sessionId must return 404 Not Found (TD-07 fixed)
    let resp_unknown = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/mcp/message?sessionId=nonexistent")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_unknown.status(), StatusCode::NOT_FOUND);

    // 2. Establish SSE session, read initial endpoint event, saturate capacity=1 buffer -> 503
    let sse_resp = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/mcp/sse")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(sse_resp.status(), StatusCode::OK);
    assert_eq!(sessions.active_sessions_count(), 1);

    let mut data_stream = sse_resp.into_body().into_data_stream();
    let first_chunk = data_stream.next().await.unwrap().unwrap();
    let first_text = String::from_utf8_lossy(&first_chunk);
    let session_id = first_text
        .split("sessionId=")
        .nth(1)
        .unwrap()
        .trim()
        .to_string();

    // Buffer capacity is 1 and `endpoint` event wasn't pulled? No, we just pulled `first_chunk`, so buffer is empty!
    // Sending first POST fills buffer (capacity 1) -> 202 Accepted; sending second unread POST -> 503 Service Unavailable!
    let msg_req = || {
        Request::builder()
            .method(Method::POST)
            .uri(format!("/mcp/message?sessionId={session_id}"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(r#"{"jsonrpc":"2.0","id":9,"method":"ping"}"#))
            .unwrap()
    };

    let r1 = router.clone().oneshot(msg_req()).await.unwrap();
    assert_eq!(r1.status(), StatusCode::ACCEPTED);
    let r2 = router.clone().oneshot(msg_req()).await.unwrap();
    assert_eq!(r2.status(), StatusCode::SERVICE_UNAVAILABLE);

    // Dropping the SSE response stream removes the session synchronously!
    drop(data_stream);
    assert_eq!(sessions.active_sessions_count(), 0);

    kernel.shutdown().await;
}

#[tokio::test]
async fn test_live_yaml_hot_reload_and_malformed_rollback() {
    let temp = TempDir::new().unwrap();
    let holidays_dir = temp.path().join("holidays");
    fs::create_dir_all(&holidays_dir).unwrap();
    fs::copy("config/cordis.yml", temp.path().join("cordis.yml")).unwrap();
    fs::copy(
        "config/cordis.patch.yml",
        temp.path().join("cordis.patch.yml"),
    )
    .unwrap();
    fs::copy(
        "config/retail_events.yml",
        temp.path().join("retail_events.yml"),
    )
    .unwrap();
    fs::copy("config/holidays/2026.yml", holidays_dir.join("2026.yml")).unwrap();

    let kernel = boot_profile(temp.path(), "serve", None, Some(false))
        .await
        .unwrap();
    let factories = Arc::new(build_default_factories());
    let anchor = RuntimeConfigAnchor {
        kernel: kernel.clone(),
        factories,
        config_dir: temp.path().to_path_buf(),
        profile: "serve".to_string(),
        port_override: None,
        bind_override: Some(false),
    };

    let hol = kernel.inject::<dyn HolidayService>().unwrap();
    let may1_2031 = NaiveDate::from_ymd_opt(2031, 5, 1).unwrap(); // Thursday
    assert!(!hol.has_schedule(2031));
    assert!(!hol.is_holiday(may1_2031));

    // Write valid 2031.yml and disable http_holidays via cordis.patch.yml; watch notify auto-reload.
    fs::write(
        holidays_dir.join("2031.yml"),
        r#"year: 2031
source: "hot-reload-test"
holidays:
  - date: "2031-05-01"
    name: "International Workers' Day"
    category: FixedSolar
working_weekends: []
"#,
    )
    .unwrap();
    fs::write(
        temp.path().join("cordis.patch.yml"),
        r#"patches:
  - id: http_holidays
    enabled: false
"#,
    )
    .unwrap();

    let mut reloaded = false;
    for _ in 0..30 {
        if hol.has_schedule(2031) && kernel.fiber_state("http_holidays").is_none() {
            reloaded = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(
        reloaded,
        "expected notify watcher to trigger debounced hot-reload"
    );
    assert!(hol.is_holiday(may1_2031));
    assert_eq!(
        kernel.fiber_state("http_calendar"),
        Some(FiberState::Active)
    );

    // Now write malformed YAML to 2031.yml -> reload_from_disk returns Err and keeps old 2031 schedule intact!
    fs::write(holidays_dir.join("2031.yml"), "year: [not_valid_yaml").unwrap();
    assert!(reload_from_disk(&anchor, "holidays").await.is_err());
    assert!(hol.has_schedule(2031));
    assert!(hol.is_holiday(may1_2031));

    kernel.shutdown().await;
}

#[tokio::test]
async fn test_niche_plugins_http_openapi_mcp_and_cascade_dispose() {
    let kernel = boot_profile(Path::new("config"), "serve", None, Some(false))
        .await
        .unwrap();
    let router_svc = kernel.inject::<dyn HttpRouterService>().unwrap();
    let mcp_svc = kernel.inject::<dyn McpRegistryService>().unwrap();
    let router = router_svc.current_router();

    // 1. All 10 new HTTP REST routes
    for uri in [
        "/canchi/2024-02-10?hour=12",
        "/solar-terms/2024",
        "/astrology/bazi/2024-02-04?hour=20&gender=male",
        "/trach-nhat/day/2024-02-10?occasion=grand_opening&owner_birth_year=1990",
        "/retail/year/2025?category=shopping",
        "/retail/upcoming?from=2025-01-20&days=30",
    ] {
        let resp = router
            .clone()
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK, "failed GET {uri}");
    }

    for (uri, payload, expected_ct) in [
        (
            "/astrology/chart",
            json!({ "date": "1990-05-15", "hour": 10, "gender": "male", "system": "both" }),
            "application/json",
        ),
        (
            "/trach-nhat/search",
            json!({ "start_date": "2024-02-10", "end_date": "2024-02-20", "occasion": "wedding" }),
            "application/json",
        ),
        (
            "/recurrence/expand",
            json!({ "preset": "new_and_full_moon", "start_date": "2025-01-01", "end_date": "2025-03-31" }),
            "application/json",
        ),
        (
            "/recurrence/ics",
            json!({ "preset": "vegetarian_four_days", "start_date": "2025-01-01", "end_date": "2025-02-28", "reminder_minutes": 60 }),
            "text/calendar",
        ),
    ] {
        let resp = router
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(uri)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK, "failed POST {uri}");
        let ct = resp
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(
            ct.starts_with(expected_ct),
            "unexpected Content-Type {ct} on {uri}"
        );
    }

    // HTTP 400 validation errors
    let bad_http = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/retail/upcoming?from=2025-01-01&days=400")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(bad_http.status(), StatusCode::BAD_REQUEST);

    // 2. OpenAPI paths registered cleanly without breaking LunarDate invariant
    let openapi_resp = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api-docs/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let openapi = body_json(openapi_resp.into_body()).await;
    for p in [
        "/canchi/{date}",
        "/solar-terms/{year}",
        "/astrology/chart",
        "/astrology/bazi/{date}",
        "/trach-nhat/day/{date}",
        "/trach-nhat/search",
        "/recurrence/expand",
        "/recurrence/ics",
        "/retail/year/{year}",
        "/retail/upcoming",
    ] {
        assert!(openapi["paths"][p].is_object(), "missing OpenAPI path {p}");
    }

    // 3. MCP 16 tools + 2 niche resource templates + negative -32602 check
    let tools_resp = mcp_svc
        .dispatch(vietcalendar_rs::contracts::mcp::JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(10)),
            method: "tools/list".into(),
            params: None,
        })
        .unwrap();
    let tools = tools_resp.result.unwrap()["tools"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(tools.len(), 16);

    for (name, args) in [
        ("get_can_chi", json!({ "date": "2024-02-10", "hour": 12 })),
        ("get_solar_terms", json!({ "year": 2024 })),
        (
            "get_active_solar_term",
            json!({ "date": "2024-02-04", "hour": 20 }),
        ),
        (
            "generate_bazi_chart",
            json!({ "date": "2024-02-04", "hour": 20, "gender": "male" }),
        ),
        (
            "generate_tuvi_chart",
            json!({ "date": "1990-05-15", "hour": 10, "gender": "male" }),
        ),
        (
            "analyze_auspicious_date",
            json!({ "date": "2024-02-10", "occasion": "grand_opening" }),
        ),
        (
            "find_auspicious_dates",
            json!({ "start_date": "2024-02-10", "end_date": "2024-02-20", "occasion": "wedding" }),
        ),
        (
            "expand_lunar_recurrence",
            json!({ "preset": "new_and_full_moon", "start_date": "2025-01-01", "end_date": "2025-03-31" }),
        ),
        (
            "export_lunar_ics",
            json!({ "preset": "new_and_full_moon", "start_date": "2025-01-01", "end_date": "2025-02-28" }),
        ),
        ("get_retail_calendar", json!({ "year": 2025 })),
        (
            "get_upcoming_retail_peaks",
            json!({ "from_date": "2025-01-20", "days": 30 }),
        ),
    ] {
        let call = mcp_svc
            .dispatch(vietcalendar_rs::contracts::mcp::JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(11)),
                method: "tools/call".into(),
                params: Some(json!({ "name": name, "arguments": args })),
            })
            .unwrap();
        assert!(
            call.error.is_none(),
            "MCP tool {name} errored: {:?}",
            call.error
        );
    }

    for uri in ["canchi://solar-terms/2024", "retail://calendar/2025"] {
        let read = mcp_svc
            .dispatch(vietcalendar_rs::contracts::mcp::JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(12)),
                method: "resources/read".into(),
                params: Some(json!({ "uri": uri })),
            })
            .unwrap();
        assert!(read.error.is_none(), "resource read failed for {uri}");
    }

    let bad_mcp = mcp_svc
        .dispatch(vietcalendar_rs::contracts::mcp::JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(13)),
            method: "tools/call".into(),
            params: Some(
                json!({ "name": "get_upcoming_retail_peaks", "arguments": { "days": 999 } }),
            ),
        })
        .unwrap();
    assert_eq!(bad_mcp.error.unwrap().code, -32602);

    // 4. Selective core plugin disposal suspends dependent HTTP + MCP adapters cleanly
    kernel.dispose("astrology_vn").await.unwrap();
    assert!(kernel.fiber_state("astrology_vn").is_none());
    assert_eq!(
        kernel.fiber_state("http_astrology"),
        Some(FiberState::Waiting)
    );
    assert_eq!(
        kernel.fiber_state("mcp_astrology"),
        Some(FiberState::Waiting)
    );

    let gone_resp = router_svc
        .current_router()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/astrology/chart")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"date":"1990-05-15","hour":10,"gender":"male"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(gone_resp.status(), StatusCode::NOT_FOUND);

    let still_ok = router_svc
        .current_router()
        .oneshot(
            Request::builder()
                .uri("/canchi/2024-02-10")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(still_ok.status(), StatusCode::OK);

    let after_tools = mcp_svc
        .dispatch(vietcalendar_rs::contracts::mcp::JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(14)),
            method: "tools/list".into(),
            params: None,
        })
        .unwrap();
    let remaining: Vec<String> = after_tools.result.unwrap()["tools"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|t| t["name"].as_str().map(str::to_string))
        .collect();
    assert_eq!(remaining.len(), 14);
    assert!(!remaining
        .iter()
        .any(|n| n == "generate_bazi_chart" || n == "generate_tuvi_chart"));

    kernel.shutdown().await;
}

#[tokio::test]
async fn test_agent_discovery_llms_txt_and_well_known_mcp() {
    // 1. `serve` profile exposes dynamic /llms.txt, /llms-full.txt, /.well-known/mcp.json & initialize.instructions
    let kernel = boot_profile(Path::new("config"), "serve", None, Some(false))
        .await
        .unwrap();
    let router_svc = kernel.inject::<dyn HttpRouterService>().unwrap();
    let router = router_svc.current_router();

    let llms_resp = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/llms.txt")
                .header(header::HOST, "lunar.example.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(llms_resp.status(), StatusCode::OK);
    let llms_ct = llms_resp
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    assert!(llms_ct.starts_with("text/markdown"));
    let llms_text = String::from_utf8(body_bytes(llms_resp.into_body()).await).unwrap();
    assert!(llms_text.contains("http://lunar.example.com/mcp"));
    assert!(llms_text.contains("generate_tuvi_chart"));
    assert!(llms_text.contains("canchi://solar-terms/{year}"));
    assert!(llms_text.contains("/astrology/chart"));

    let full_resp = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/llms-full.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(full_resp.status(), StatusCode::OK);
    let full_text = String::from_utf8(body_bytes(full_resp.into_body()).await).unwrap();
    assert!(full_text.contains("inputSchema") || full_text.contains("\"properties\""));
    assert!(full_text.contains("get_upcoming_retail_peaks"));

    let wk_resp = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/.well-known/mcp.json")
                .header(header::HOST, "lunar.example.com")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(wk_resp.status(), StatusCode::OK);
    let wk = body_json(wk_resp.into_body()).await;
    assert_eq!(wk["mcp_enabled"], true);
    assert_eq!(wk["protocol_version"], "2024-11-05");
    assert_eq!(wk["tools"].as_array().unwrap().len(), 16);
    assert_eq!(wk["resource_templates"].as_array().unwrap().len(), 3);
    assert!(wk["instructions"].as_str().unwrap().contains("UTC+7"));

    // Direct POST /mcp initialize emits live instructions
    let init_resp = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/mcp")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"jsonrpc":"2.0","id":99,"method":"initialize","params":{}}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let init_json = body_json(init_resp.into_body()).await;
    let init_inst = init_json["result"]["instructions"].as_str().unwrap();
    assert!(init_inst.contains("generate_tuvi_chart"));
    assert!(init_inst.contains("retail://calendar/{year}"));

    // 2. Disposing astrology_vn live drops both /astrology/* REST routes & generate_tuvi_chart from discovery
    kernel.dispose("astrology_vn").await.unwrap();
    let wk_after = body_json(
        router_svc
            .current_router()
            .oneshot(
                Request::builder()
                    .uri("/.well-known/mcp.json")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap()
            .into_body(),
    )
    .await;
    assert_eq!(wk_after["tools"].as_array().unwrap().len(), 14);
    assert!(!wk_after["tools"]
        .as_array()
        .unwrap()
        .iter()
        .any(|t| t["name"] == "generate_tuvi_chart"));
    assert!(!wk_after["http_endpoints"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["url"] == "/astrology/chart"));

    kernel.shutdown().await;

    // 3. `http-only` profile reports mcp_enabled: false and omits MCP section in /llms.txt
    let http_kernel = boot_profile(Path::new("config"), "http-only", None, Some(false))
        .await
        .unwrap();
    let http_router = http_kernel
        .inject::<dyn HttpRouterService>()
        .unwrap()
        .current_router();
    let wk_http = body_json(
        http_router
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/.well-known/mcp.json")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap()
            .into_body(),
    )
    .await;
    assert_eq!(wk_http["mcp_enabled"], false);
    assert!(wk_http["transports"].is_null());
    assert_eq!(wk_http["tools"].as_array().unwrap().len(), 0);
    assert!(wk_http["http_endpoints"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["url"] == "/vietnam-holiday"));

    let llms_http = String::from_utf8(
        body_bytes(
            http_router
                .oneshot(
                    Request::builder()
                        .uri("/llms.txt")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap()
                .into_body(),
        )
        .await,
    )
    .unwrap();
    assert!(!llms_http.contains("Streamable HTTP"));
    assert!(llms_http.contains("/convert/solar-to-lunar/{date}"));

    http_kernel.shutdown().await;
}
