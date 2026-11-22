//! `HttpDiscoveryPlugin`: dynamically serves `/llms.txt`, `/llms-full.txt`, and `/.well-known/mcp.json`
//! from live `HttpRouterService` and optional `McpRegistryService` snapshots.

use std::sync::Arc;

use axum::extract::State;
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use utoipa::openapi::path::PathItem;
use utoipa::openapi::OpenApi as OpenApiSpec;
use utoipa::{OpenApi, ToSchema};

use crate::contracts::http::HttpRouterService;
use crate::contracts::mcp::{JsonRpcRequest, McpRegistryService};
use crate::kernel::context::Context;
use crate::kernel::effect::BoxFuture;
use crate::kernel::error::KernelError;
use crate::kernel::fiber::{Plugin, PluginMeta};

#[derive(Debug, Clone, Default, Deserialize)]
struct DiscoveryConfig {
    #[serde(default)]
    base_url: Option<String>,
}

#[derive(Clone)]
struct DiscoveryState {
    ctx: Context,
    router_svc: Arc<dyn HttpRouterService>,
    configured_base_url: Option<String>,
}

/// Validates and normalizes `scheme` (`http` / `https`) and `host` into a safe origin URL.
///
/// Returns an empty string when `host` is absent, oversized, or contains unsafe characters.
pub fn sanitize_origin(proto: Option<&str>, host: Option<&str>) -> String {
    let Some(raw_host) = host else {
        return String::new();
    };
    if raw_host.is_empty() || raw_host.len() > 253 {
        return String::new();
    }
    let valid_host_chars = raw_host.bytes().all(|b| {
        matches!(
            b,
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'.' | b'-' | b':' | b'[' | b']'
        )
    });
    if !valid_host_chars {
        return String::new();
    }

    let scheme = match proto {
        Some(p) if p.eq_ignore_ascii_case("https") => "https",
        _ => "http",
    };
    format!("{scheme}://{raw_host}")
}

fn resolve_origin(headers: &HeaderMap, configured_base_url: Option<&str>) -> String {
    if let Some(url) = configured_base_url {
        let trimmed = url.trim().trim_end_matches('/');
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    let proto = headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(str::trim);
    let host = headers
        .get("x-forwarded-host")
        .or_else(|| headers.get(header::HOST))
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(str::trim);
    sanitize_origin(proto, host)
}

/// Summary of an active MCP tool advertised in discovery responses.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DiscoveryToolSummary {
    /// Tool name passed to `tools/call`.
    pub name: String,
    /// Human/agent-readable tool purpose.
    pub description: String,
    /// JSON Schema for tool arguments.
    #[serde(skip_serializing_if = "Value::is_null")]
    pub input_schema: Value,
}

/// Summary of an active concrete MCP resource.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DiscoveryResourceSummary {
    /// Concrete resource URI.
    pub uri: String,
    /// Resource display title.
    pub name: String,
    /// Resource description.
    pub description: String,
    /// MIME media type (typically `application/json`).
    pub mime_type: String,
}

/// Summary of an active RFC 6570 MCP resource URI template.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DiscoveryTemplateSummary {
    /// RFC 6570 URI template pattern.
    pub uri_template: String,
    /// Template display title.
    pub name: String,
    /// Template description.
    pub description: String,
    /// MIME media type.
    pub mime_type: String,
}

/// Summary of a mounted HTTP REST operation extracted from OpenAPI.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct HttpEndpointSummary {
    /// HTTP method (`GET`, `POST`, etc.).
    pub method: &'static str,
    /// Full or root-relative endpoint URL.
    pub url: String,
    /// Short operation summary.
    pub description: String,
}

/// Available MCP transport endpoints and CLI commands.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct McpTransportsDoc {
    /// Direct Streamable HTTP JSON-RPC POST transport.
    pub streamable_http: McpTransportEndpoint,
    /// Legacy Server-Sent Events stream + message POST transport.
    pub sse: McpSseEndpoint,
    /// Standard I/O CLI commands for local MCP hosts.
    pub stdio: McpStdioCommands,
}

/// Direct HTTP JSON-RPC POST endpoint descriptor.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct McpTransportEndpoint {
    /// Endpoint URL for JSON-RPC POST requests.
    pub url: String,
    /// HTTP verb (`POST`).
    pub method: &'static str,
}

/// Legacy SSE + message POST endpoint pair.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct McpSseEndpoint {
    /// GET SSE stream URL.
    pub sse_url: String,
    /// POST message URL requiring `?sessionId=...`.
    pub message_url: String,
}

/// Binary and subcommand entrypoints for local stdio MCP clients.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct McpStdioCommands {
    /// Dedicated stdio MCP binary name.
    pub binary: &'static str,
    /// Subcommand form on the main `vietcalendar` binary.
    pub alt_subcommand: &'static str,
}

/// JSON discovery descriptor served at `GET /.well-known/mcp.json`.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct McpDiscoveryDocument {
    /// Service title from active OpenAPI metadata.
    pub name: String,
    /// Service version from active OpenAPI metadata.
    pub version: String,
    /// Service summary description.
    pub description: String,
    /// `true` when `McpRegistryService` is mounted in the active profile.
    pub mcp_enabled: bool,
    /// Supported Model Context Protocol revision (`"2024-11-05"`).
    pub protocol_version: &'static str,
    /// Active MCP instructions returned during `initialize`, if MCP is mounted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
    /// MCP transport endpoints when `mcp_enabled == true`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transports: Option<McpTransportsDoc>,
    /// Active MCP tools dynamically discovered from `McpRegistryService`.
    pub tools: Vec<DiscoveryToolSummary>,
    /// Active concrete MCP resources.
    pub resources: Vec<DiscoveryResourceSummary>,
    /// Active RFC 6570 MCP resource URI templates.
    pub resource_templates: Vec<DiscoveryTemplateSummary>,
    /// Active REST endpoints dynamically extracted from `HttpRouterService::current_openapi()`.
    pub http_endpoints: Vec<HttpEndpointSummary>,
    /// Companion `llms.txt`, `llms-full.txt`, OpenAPI, and Swagger UI URLs.
    pub links: Value,
}

struct McpLiveSnapshot {
    enabled: bool,
    instructions: Option<String>,
    tools: Vec<DiscoveryToolSummary>,
    resources: Vec<DiscoveryResourceSummary>,
    templates: Vec<DiscoveryTemplateSummary>,
}

fn dispatch_list(reg: &dyn McpRegistryService, method: &str) -> Option<Value> {
    reg.dispatch(JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: method.to_string(),
        params: None,
    })
    .and_then(|resp| resp.result)
}

fn collect_mcp_snapshot(ctx: &Context) -> McpLiveSnapshot {
    let Some(reg) = ctx.try_inject::<dyn McpRegistryService>() else {
        return McpLiveSnapshot {
            enabled: false,
            instructions: None,
            tools: Vec::new(),
            resources: Vec::new(),
            templates: Vec::new(),
        };
    };

    let instructions = dispatch_list(reg.as_ref(), "initialize").and_then(|v| {
        v.get("instructions")
            .and_then(Value::as_str)
            .map(str::to_string)
    });

    let tools = dispatch_list(reg.as_ref(), "tools/list")
        .and_then(|v| v.get("tools").and_then(Value::as_array).cloned())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|item| {
            let name = item.get("name")?.as_str()?.to_string();
            let description = item
                .get("description")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let input_schema = item.get("inputSchema").cloned().unwrap_or(Value::Null);
            Some(DiscoveryToolSummary {
                name,
                description,
                input_schema,
            })
        })
        .collect();

    let resources = dispatch_list(reg.as_ref(), "resources/list")
        .and_then(|v| v.get("resources").and_then(Value::as_array).cloned())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|item| {
            let uri = item.get("uri")?.as_str()?.to_string();
            let name = item
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let description = item
                .get("description")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let mime_type = item
                .get("mimeType")
                .and_then(Value::as_str)
                .unwrap_or("application/json")
                .to_string();
            Some(DiscoveryResourceSummary {
                uri,
                name,
                description,
                mime_type,
            })
        })
        .collect();

    let templates = dispatch_list(reg.as_ref(), "resources/templates/list")
        .and_then(|v| {
            v.get("resourceTemplates")
                .and_then(Value::as_array)
                .cloned()
        })
        .unwrap_or_default()
        .into_iter()
        .filter_map(|item| {
            let uri_template = item.get("uriTemplate")?.as_str()?.to_string();
            let name = item
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let description = item
                .get("description")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let mime_type = item
                .get("mimeType")
                .and_then(Value::as_str)
                .unwrap_or("application/json")
                .to_string();
            Some(DiscoveryTemplateSummary {
                uri_template,
                name,
                description,
                mime_type,
            })
        })
        .collect();

    McpLiveSnapshot {
        enabled: true,
        instructions,
        tools,
        resources,
        templates,
    }
}

fn collect_http_endpoints(openapi: &OpenApiSpec, origin: &str) -> Vec<HttpEndpointSummary> {
    let mut endpoints = Vec::new();
    for (path, item) in &openapi.paths.paths {
        if matches!(
            path.as_str(),
            "/llms.txt" | "/llms-full.txt" | "/.well-known/mcp.json"
        ) {
            continue;
        }
        collect_item_ops(path, origin, item, &mut endpoints);
    }
    endpoints
}

fn collect_item_ops(path: &str, origin: &str, item: &PathItem, out: &mut Vec<HttpEndpointSummary>) {
    let ops = [
        ("GET", item.get.as_ref()),
        ("POST", item.post.as_ref()),
        ("PUT", item.put.as_ref()),
        ("DELETE", item.delete.as_ref()),
        ("PATCH", item.patch.as_ref()),
    ];
    for (method, op_opt) in ops {
        if let Some(op) = op_opt {
            let desc = op
                .summary
                .as_deref()
                .or(op.description.as_deref())
                .unwrap_or("REST API operation")
                .trim()
                .replace('\n', " ");
            out.push(HttpEndpointSummary {
                method,
                url: format!("{origin}{path}"),
                description: desc,
            });
        }
    }
}

fn render_llms_markdown(
    openapi: &OpenApiSpec,
    mcp: &McpLiveSnapshot,
    endpoints: &[HttpEndpointSummary],
    origin: &str,
    full_schemas: bool,
) -> String {
    let mut md = String::with_capacity(4096);
    let summary = openapi.info.description.as_deref().unwrap_or(
        "Deterministic Vietnamese Luni-Solar calendar, holiday, and cultural intelligence server.",
    );

    md.push_str("# ");
    md.push_str(&openapi.info.title);
    md.push_str("\n\n> ");
    md.push_str(summary);
    md.push_str("\n\n");
    md.push_str(
        "All calendar, holiday, Can Chi, 24 Solar Terms, BaZi/Tử Vi, Trạch Nhật, lunar recurrence, and retail season computations are anchored to Vietnam Indochina Time (UTC+7) unless a timezone is specified.\n\n",
    );

    md.push_str("## Discovery & Docs\n\n");
    md.push_str(&format!(
        "- [Full Agent Reference (with schemas)]({origin}/llms-full.txt)\n"
    ));
    md.push_str(&format!(
        "- [MCP Machine Manifest]({origin}/.well-known/mcp.json)\n"
    ));
    md.push_str(&format!(
        "- [OpenAPI 3.0 JSON Specification]({origin}/api-docs/openapi.json)\n"
    ));
    md.push_str(&format!(
        "- [Interactive Swagger UI]({origin}/swagger-ui)\n\n"
    ));

    if mcp.enabled {
        md.push_str("## Model Context Protocol (MCP 2024-11-05)\n\n");
        md.push_str(&format!("- Streamable HTTP POST: `{origin}/mcp`\n"));
        md.push_str(&format!("- Legacy HTTP/SSE Stream: `{origin}/mcp/sse`\n"));
        md.push_str("- Stdio Binary: `vietcalendar-mcp` (or `vietcalendar mcp`)\n\n");

        if !mcp.tools.is_empty() {
            md.push_str(&format!("### MCP Tools ({})\n\n", mcp.tools.len()));
            for tool in &mcp.tools {
                md.push_str(&format!("- `{}`: {}\n", tool.name, tool.description));
                if full_schemas && !tool.input_schema.is_null() {
                    if let Ok(pretty) = serde_json::to_string_pretty(&tool.input_schema) {
                        md.push_str("  ```json\n");
                        for line in pretty.lines() {
                            md.push_str("  ");
                            md.push_str(line);
                            md.push('\n');
                        }
                        md.push_str("  ```\n");
                    }
                }
            }
            md.push('\n');
        }

        if !mcp.resources.is_empty() || !mcp.templates.is_empty() {
            md.push_str("### MCP Resources & Templates\n\n");
            for res in &mcp.resources {
                md.push_str(&format!(
                    "- `{}` ({}): {}\n",
                    res.uri, res.name, res.description
                ));
            }
            for tmpl in &mcp.templates {
                md.push_str(&format!(
                    "- `{}` ({}): {}\n",
                    tmpl.uri_template, tmpl.name, tmpl.description
                ));
            }
            md.push('\n');
        }
    }

    if !endpoints.is_empty() {
        md.push_str("## HTTP REST Endpoints\n\n");
        for ep in endpoints {
            md.push_str(&format!(
                "- `{}` [{}]({}): {}\n",
                ep.method, ep.url, ep.url, ep.description
            ));
        }
        md.push('\n');
    }

    md
}

#[derive(OpenApi)]
#[openapi(paths(get_llms_txt, get_llms_full_txt, get_well_known_mcp))]
struct DiscoveryApiDoc;

/// Serve concise `llmstxt.org`-formatted Markdown index reflecting live routes & MCP capabilities.
#[utoipa::path(
    get,
    path = "/llms.txt",
    responses(
        (status = 200, description = "Dynamic llms.txt Markdown index for AI agents", content_type = "text/markdown", body = String)
    )
)]
async fn get_llms_txt(
    State(state): State<Arc<DiscoveryState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let origin = resolve_origin(&headers, state.configured_base_url.as_deref());
    let openapi = state.router_svc.current_openapi();
    let mcp = collect_mcp_snapshot(&state.ctx);
    let endpoints = collect_http_endpoints(&openapi, &origin);
    let body = render_llms_markdown(&openapi, &mcp, &endpoints, &origin, false);
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/markdown; charset=utf-8")],
        body,
    )
}

/// Serve full Markdown reference including complete JSON `inputSchema` definitions for all live MCP tools.
#[utoipa::path(
    get,
    path = "/llms-full.txt",
    responses(
        (status = 200, description = "Full llms-full.txt Markdown reference with JSON schemas", content_type = "text/markdown", body = String)
    )
)]
async fn get_llms_full_txt(
    State(state): State<Arc<DiscoveryState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let origin = resolve_origin(&headers, state.configured_base_url.as_deref());
    let openapi = state.router_svc.current_openapi();
    let mcp = collect_mcp_snapshot(&state.ctx);
    let endpoints = collect_http_endpoints(&openapi, &origin);
    let body = render_llms_markdown(&openapi, &mcp, &endpoints, &origin, true);
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/markdown; charset=utf-8")],
        body,
    )
}

/// Serve machine-readable JSON discovery manifest of live MCP transports, tools, resources, and REST endpoints.
#[utoipa::path(
    get,
    path = "/.well-known/mcp.json",
    responses(
        (status = 200, description = "Machine-readable live MCP & REST discovery manifest", body = McpDiscoveryDocument)
    )
)]
async fn get_well_known_mcp(
    State(state): State<Arc<DiscoveryState>>,
    headers: HeaderMap,
) -> Json<McpDiscoveryDocument> {
    let origin = resolve_origin(&headers, state.configured_base_url.as_deref());
    let openapi = state.router_svc.current_openapi();
    let mcp = collect_mcp_snapshot(&state.ctx);
    let http_endpoints = collect_http_endpoints(&openapi, &origin);

    let transports = if mcp.enabled {
        Some(McpTransportsDoc {
            streamable_http: McpTransportEndpoint {
                url: format!("{origin}/mcp"),
                method: "POST",
            },
            sse: McpSseEndpoint {
                sse_url: format!("{origin}/mcp/sse"),
                message_url: format!("{origin}/mcp/message"),
            },
            stdio: McpStdioCommands {
                binary: "vietcalendar-mcp",
                alt_subcommand: "vietcalendar mcp",
            },
        })
    } else {
        None
    };

    Json(McpDiscoveryDocument {
        name: openapi.info.title.clone(),
        version: openapi.info.version.clone(),
        description: openapi.info.description.clone().unwrap_or_default(),
        mcp_enabled: mcp.enabled,
        protocol_version: "2024-11-05",
        instructions: mcp.instructions,
        transports,
        tools: mcp.tools,
        resources: mcp.resources,
        resource_templates: mcp.templates,
        http_endpoints,
        links: json!({
            "llms_txt": format!("{origin}/llms.txt"),
            "llms_full_txt": format!("{origin}/llms-full.txt"),
            "openapi_json": format!("{origin}/api-docs/openapi.json"),
            "swagger_ui": format!("{origin}/swagger-ui")
        }),
    })
}

/// Cordis plugin serving live agent discovery metadata (`/llms.txt`, `/llms-full.txt`, `/.well-known/mcp.json`).
#[derive(Debug, Default)]
pub struct HttpDiscoveryPlugin;

impl Plugin for HttpDiscoveryPlugin {
    fn meta(&self) -> PluginMeta {
        PluginMeta::new("http_discovery").requires::<dyn HttpRouterService>()
    }

    fn mount<'a>(&'a self, ctx: Context, config: Value) -> BoxFuture<'a, Result<(), KernelError>> {
        Box::pin(async move {
            let cfg: DiscoveryConfig = serde_json::from_value(config)
                .map_err(|e| KernelError::ConfigError(format!("http_discovery config: {e}")))?;
            let router_svc = ctx.inject::<dyn HttpRouterService>()?;
            let state = Arc::new(DiscoveryState {
                ctx: ctx.clone(),
                router_svc: Arc::clone(&router_svc),
                configured_base_url: cfg.base_url,
            });
            let router = Router::new()
                .route("/llms.txt", get(get_llms_txt))
                .route("/llms-full.txt", get(get_llms_full_txt))
                .route("/.well-known/mcp.json", get(get_well_known_mcp))
                .with_state(state);
            let disposer = router_svc.mount_routes(router, Some(DiscoveryApiDoc::openapi()));
            ctx.on_dispose_sync(disposer);
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::body::Body;
    use axum::http::{header, Request, StatusCode};
    use http_body_util::BodyExt;
    use serde_json::{json, Value};
    use tower::ServiceExt;

    use crate::contracts::http::HttpRouterService;
    use crate::kernel::Kernel;
    use crate::plugins::calendar_vn::VietnamCalendarPlugin;
    use crate::plugins::holidays_vn::VietnamHolidaysPlugin;
    use crate::plugins::http_calendar::HttpCalendarPlugin;
    use crate::plugins::http_discovery::{sanitize_origin, HttpDiscoveryPlugin};
    use crate::plugins::http_server::HttpServerPlugin;
    use crate::plugins::mcp_core::McpCorePlugin;
    use crate::plugins::mcp_tools::McpToolsPlugin;
    use crate::plugins::openapi::OpenApiPlugin;

    async fn read_text(resp: axum::response::Response) -> (StatusCode, String, String) {
        let status = resp.status();
        let ct = resp
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        (status, ct, String::from_utf8_lossy(&bytes).into_owned())
    }

    #[test]
    fn test_sanitize_origin_rejects_malicious_headers() {
        let cases = [
            (
                Some("https"),
                Some("api.example.com:8443"),
                "https://api.example.com:8443",
            ),
            (Some("http"), Some("[::1]:8080"), "http://[::1]:8080"),
            (
                Some("HTTP"),
                Some("localhost:8080"),
                "http://localhost:8080",
            ),
            (
                Some("javascript"),
                Some("localhost:8080"),
                "http://localhost:8080",
            ),
            (None, Some("vietcalendar.dev"), "http://vietcalendar.dev"),
            (Some("https"), None, ""),
            (Some("https"), Some(""), ""),
            (Some("https"), Some("evil.com/path"), ""),
            (Some("https"), Some("evil.com)\n# injected"), ""),
            (Some("https"), Some("evil com"), ""),
        ];
        for (proto, host, expected) in cases {
            assert_eq!(
                sanitize_origin(proto, host),
                expected,
                "failed for proto={proto:?} host={host:?}"
            );
        }
        let too_long = "a".repeat(254);
        assert_eq!(sanitize_origin(Some("https"), Some(&too_long)), "");
        let max_valid = "a".repeat(253);
        assert_eq!(
            sanitize_origin(Some("https"), Some(&max_valid)),
            format!("https://{max_valid}")
        );
    }

    #[tokio::test]
    async fn test_llms_txt_and_well_known_reflect_live_plugins_and_unmount() {
        let kernel = Kernel::new();
        kernel
            .mount("calendar_vn", Arc::new(VietnamCalendarPlugin), json!({}))
            .await
            .unwrap();
        kernel
            .mount(
                "holidays_vn",
                Arc::new(VietnamHolidaysPlugin),
                json!({ "holidays_dir": "config/holidays" }),
            )
            .await
            .unwrap();
        kernel
            .mount(
                "http_server",
                Arc::new(HttpServerPlugin),
                json!({ "bind": false }),
            )
            .await
            .unwrap();
        kernel
            .mount("http_calendar", Arc::new(HttpCalendarPlugin), json!({}))
            .await
            .unwrap();
        kernel
            .mount(
                "openapi",
                Arc::new(OpenApiPlugin),
                json!({ "title": "VietCalendar Test API", "version": "0.2.0", "description": "Test desc" }),
            )
            .await
            .unwrap();
        kernel
            .mount("mcp_core", Arc::new(McpCorePlugin), json!({}))
            .await
            .unwrap();
        kernel
            .mount("mcp_tools", Arc::new(McpToolsPlugin), json!({}))
            .await
            .unwrap();
        kernel
            .mount("http_discovery", Arc::new(HttpDiscoveryPlugin), json!({}))
            .await
            .unwrap();

        let router_svc = kernel.inject::<dyn HttpRouterService>().unwrap();
        let router = router_svc.current_router();

        // 1. /llms.txt with trusted reverse-proxy headers
        let (st, ct, body) = read_text(
            router
                .clone()
                .oneshot(
                    Request::builder()
                        .uri("/llms.txt")
                        .header("x-forwarded-proto", "https")
                        .header(header::HOST, "cal.local:8080")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(st, StatusCode::OK);
        assert!(ct.starts_with("text/markdown"), "got content-type: {ct}");
        assert!(body.starts_with("# VietCalendar Test API"));
        assert!(body.contains("/convert/solar-to-lunar/{date}"));
        assert!(body.contains("get_today_lunar"));
        assert!(body.contains("calendar://today"));
        assert!(body.contains("calendar://holidays/{year}"));
        assert!(body.contains("https://cal.local:8080/mcp"));
        assert!(!body.contains("- `GET` [http://cal.local:8080/llms.txt]"));
        assert!(!body.contains("- `GET` [https://cal.local:8080/llms.txt]"));

        // 2. /llms-full.txt includes JSON inputSchema details and omits raw discovery routes
        let (st_full, ct_full, full_body) = read_text(
            router
                .clone()
                .oneshot(
                    Request::builder()
                        .uri("/llms-full.txt")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(st_full, StatusCode::OK);
        assert!(ct_full.starts_with("text/markdown"));
        assert!(full_body.contains("\"properties\""));
        assert!(full_body.contains("convert_solar_to_lunar"));
        assert!(!full_body.contains("- `GET` [/llms.txt]"));

        // 3. /.well-known/mcp.json rejects malformed Host and reports all live metadata
        let (st_wk, ct_wk, wk_body) = read_text(
            router
                .clone()
                .oneshot(
                    Request::builder()
                        .uri("/.well-known/mcp.json")
                        .header(header::HOST, "bad host)X")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(st_wk, StatusCode::OK);
        assert!(ct_wk.starts_with("application/json"));
        let wk: Value = serde_json::from_str(&wk_body).unwrap();
        assert_eq!(wk["name"], "VietCalendar Test API");
        assert_eq!(wk["mcp_enabled"], true);
        assert_eq!(wk["protocol_version"], "2024-11-05");
        assert!(wk["instructions"].as_str().unwrap().contains("UTC+7"));
        assert_eq!(wk["transports"]["streamable_http"]["url"], "/mcp");
        assert!(wk["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t["name"] == "get_today_lunar"));
        assert!(wk["resources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["uri"] == "calendar://today"));
        assert!(wk["resource_templates"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t["uri_template"] == "calendar://holidays/{year}"));
        assert!(wk["http_endpoints"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["url"] == "/convert/solar-to-lunar/{date}"));

        // 4. Disposing mcp_tools immediately removes its tools from discovery outputs
        kernel.dispose("mcp_tools").await.unwrap();
        let (_, _, wk_after_tools) = read_text(
            router_svc
                .current_router()
                .oneshot(
                    Request::builder()
                        .uri("/.well-known/mcp.json")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap(),
        )
        .await;
        let wk2: Value = serde_json::from_str(&wk_after_tools).unwrap();
        assert_eq!(wk2["mcp_enabled"], true);
        assert_eq!(wk2["tools"].as_array().unwrap().len(), 0);

        // 5. Disposing mcp_core mirrors http-only mode (mcp_enabled: false)
        kernel.dispose("mcp_core").await.unwrap();
        let (_, _, body_http_only) = read_text(
            router_svc
                .current_router()
                .oneshot(
                    Request::builder()
                        .uri("/llms.txt")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap(),
        )
        .await;
        assert!(!body_http_only.contains("Streamable HTTP"));

        let (_, _, wk_http_only) = read_text(
            router_svc
                .current_router()
                .oneshot(
                    Request::builder()
                        .uri("/.well-known/mcp.json")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap(),
        )
        .await;
        let wk3: Value = serde_json::from_str(&wk_http_only).unwrap();
        assert_eq!(wk3["mcp_enabled"], false);
        assert!(wk3["transports"].is_null());
        assert_eq!(wk3["tools"].as_array().unwrap().len(), 0);

        // 6. Configured base_url overrides Host headers
        kernel
            .reconfigure(
                "http_discovery",
                json!({ "base_url": "https://calendar.example.org/" }),
            )
            .await
            .unwrap();
        let (_, _, wk_cfg) = read_text(
            router_svc
                .current_router()
                .oneshot(
                    Request::builder()
                        .uri("/.well-known/mcp.json")
                        .header(header::HOST, "ignored.local")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap(),
        )
        .await;
        let wk4: Value = serde_json::from_str(&wk_cfg).unwrap();
        assert_eq!(
            wk4["links"]["llms_txt"],
            "https://calendar.example.org/llms.txt"
        );

        kernel.shutdown().await;
    }
}
