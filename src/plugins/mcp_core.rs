//! `McpCorePlugin`: lock-free `ArcSwap` MCP JSON-RPC 2.0 dispatcher and registry.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use arc_swap::ArcSwap;
use serde_json::{json, Value};

use crate::contracts::mcp::{
    JsonRpcRequest, JsonRpcResponse, McpDisposer, McpRegistryService, McpResourceSpec,
    McpResourceTemplateSpec, McpToolSpec,
};
use crate::kernel::context::Context;
use crate::kernel::effect::BoxFuture;
use crate::kernel::error::KernelError;
use crate::kernel::fiber::{Plugin, PluginMeta};

#[derive(Clone, Default)]
struct RegistrySnapshot {
    tools: HashMap<String, (u64, McpToolSpec)>,
    resources: HashMap<String, (u64, McpResourceSpec)>,
    templates: Vec<(u64, McpResourceTemplateSpec)>,
}

struct McpCoreInner {
    snapshot: ArcSwap<RegistrySnapshot>,
    write_lock: Mutex<()>,
    next_id: AtomicU64,
}

/// Wait-free `McpRegistryService` supporting live reversible tool and resource registration.
#[derive(Clone)]
pub struct McpCoreEngine {
    inner: Arc<McpCoreInner>,
}

impl McpCoreEngine {
    /// Creates an empty MCP dispatcher registry.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(McpCoreInner {
                snapshot: ArcSwap::from_pointee(RegistrySnapshot::default()),
                write_lock: Mutex::new(()),
                next_id: AtomicU64::new(1),
            }),
        }
    }
}

impl Default for McpCoreEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl McpRegistryService for McpCoreEngine {
    fn register_tool(&self, spec: McpToolSpec) -> McpDisposer {
        let id = self.inner.next_id.fetch_add(1, Ordering::Relaxed);
        let name = spec.name.clone();
        {
            let _guard = self
                .inner
                .write_lock
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            let mut next = (**self.inner.snapshot.load()).clone();
            next.tools.insert(name.clone(), (id, spec));
            self.inner.snapshot.store(Arc::new(next));
        }

        let inner = Arc::clone(&self.inner);
        Box::new(move || {
            let _guard = inner.write_lock.lock().unwrap_or_else(|e| e.into_inner());
            let current = inner.snapshot.load();
            if current.tools.get(&name).is_some_and(|(tid, _)| *tid == id) {
                let mut next = (**current).clone();
                next.tools.remove(&name);
                inner.snapshot.store(Arc::new(next));
            }
        })
    }

    fn register_resource(&self, spec: McpResourceSpec) -> McpDisposer {
        let id = self.inner.next_id.fetch_add(1, Ordering::Relaxed);
        let uri = spec.uri.clone();
        {
            let _guard = self
                .inner
                .write_lock
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            let mut next = (**self.inner.snapshot.load()).clone();
            next.resources.insert(uri.clone(), (id, spec));
            self.inner.snapshot.store(Arc::new(next));
        }

        let inner = Arc::clone(&self.inner);
        Box::new(move || {
            let _guard = inner.write_lock.lock().unwrap_or_else(|e| e.into_inner());
            let current = inner.snapshot.load();
            if current
                .resources
                .get(&uri)
                .is_some_and(|(rid, _)| *rid == id)
            {
                let mut next = (**current).clone();
                next.resources.remove(&uri);
                inner.snapshot.store(Arc::new(next));
            }
        })
    }

    fn register_resource_template(&self, spec: McpResourceTemplateSpec) -> McpDisposer {
        let id = self.inner.next_id.fetch_add(1, Ordering::Relaxed);
        {
            let _guard = self
                .inner
                .write_lock
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            let mut next = (**self.inner.snapshot.load()).clone();
            next.templates.push((id, spec));
            self.inner.snapshot.store(Arc::new(next));
        }

        let inner = Arc::clone(&self.inner);
        Box::new(move || {
            let _guard = inner.write_lock.lock().unwrap_or_else(|e| e.into_inner());
            let mut next = (**inner.snapshot.load()).clone();
            next.templates.retain(|(tid, _)| *tid != id);
            inner.snapshot.store(Arc::new(next));
        })
    }

    fn dispatch(&self, req: JsonRpcRequest) -> Option<JsonRpcResponse> {
        let id = req.id.clone();
        match req.method.as_str() {
            "initialize" => {
                let snap = self.inner.snapshot.load();
                let instructions = build_initialize_instructions(&snap);
                Some(JsonRpcResponse::success(
                    id,
                    json!({
                        "protocolVersion": "2024-11-05",
                        "capabilities": {
                            "tools": {},
                            "resources": {}
                        },
                        "serverInfo": {
                            "name": "vietcalendar-mcp",
                            "version": env!("CARGO_PKG_VERSION")
                        },
                        "instructions": instructions
                    }),
                ))
            }
            "notifications/initialized" => None,
            "ping" => Some(JsonRpcResponse::success(id, json!({}))),
            "tools/list" => {
                let snap = self.inner.snapshot.load();
                let mut tools: Vec<(&String, &McpToolSpec)> =
                    snap.tools.iter().map(|(k, (_, v))| (k, v)).collect();
                tools.sort_by(|a, b| a.0.cmp(b.0));
                let tool_defs: Vec<Value> = tools
                    .into_iter()
                    .map(|(_, t)| {
                        json!({
                            "name": t.name,
                            "description": t.description,
                            "inputSchema": t.input_schema
                        })
                    })
                    .collect();
                Some(JsonRpcResponse::success(id, json!({ "tools": tool_defs })))
            }
            "tools/call" => {
                let Some(params) = req.params else {
                    return Some(JsonRpcResponse::error(id, -32602, "Missing params"));
                };
                let Some(name) = params.get("name").and_then(|v| v.as_str()) else {
                    return Some(JsonRpcResponse::error(id, -32602, "Missing tool name"));
                };
                let args = params
                    .get("arguments")
                    .cloned()
                    .unwrap_or_else(|| json!({}));

                let handler = {
                    let snap = self.inner.snapshot.load();
                    snap.tools
                        .get(name)
                        .map(|(_, spec)| Arc::clone(&spec.handler))
                };
                let Some(handler) = handler else {
                    return Some(JsonRpcResponse::error(
                        id,
                        -32601,
                        format!("Tool not found: {name}"),
                    ));
                };

                match handler(args) {
                    Ok(value) => match serde_json::to_string_pretty(&value) {
                        Ok(text) => Some(JsonRpcResponse::success(
                            id,
                            json!({
                                "content": [
                                    {
                                        "type": "text",
                                        "text": text
                                    }
                                ]
                            }),
                        )),
                        Err(err) => Some(JsonRpcResponse::error(
                            id,
                            -32603,
                            format!("Failed to serialize tool response: {err}"),
                        )),
                    },
                    Err(err) => Some(JsonRpcResponse::error(id, err.code(), err.to_string())),
                }
            }
            "resources/list" => {
                let snap = self.inner.snapshot.load();
                let mut res_list: Vec<(&String, &McpResourceSpec)> =
                    snap.resources.iter().map(|(k, (_, v))| (k, v)).collect();
                res_list.sort_by(|a, b| a.0.cmp(b.0));
                let resources: Vec<Value> = res_list
                    .into_iter()
                    .map(|(_, r)| {
                        json!({
                            "uri": r.uri,
                            "name": r.name,
                            "description": r.description,
                            "mimeType": r.mime_type
                        })
                    })
                    .collect();
                Some(JsonRpcResponse::success(
                    id,
                    json!({ "resources": resources }),
                ))
            }
            "resources/templates/list" => {
                let snap = self.inner.snapshot.load();
                let templates: Vec<Value> = snap
                    .templates
                    .iter()
                    .map(|(_, t)| {
                        json!({
                            "uriTemplate": t.uri_template,
                            "name": t.name,
                            "description": t.description,
                            "mimeType": t.mime_type
                        })
                    })
                    .collect();
                Some(JsonRpcResponse::success(
                    id,
                    json!({ "resourceTemplates": templates }),
                ))
            }
            "resources/read" => {
                let Some(params) = req.params else {
                    return Some(JsonRpcResponse::error(id, -32602, "Missing params"));
                };
                let Some(uri) = params.get("uri").and_then(|v| v.as_str()) else {
                    return Some(JsonRpcResponse::error(id, -32602, "Missing 'uri'"));
                };

                let snap = self.inner.snapshot.load();
                if let Some((_, spec)) = snap.resources.get(uri) {
                    return read_resource_reply(id, uri, &spec.mime_type, (spec.handler)(uri));
                }

                for (_, tmpl) in &snap.templates {
                    if let Some(suffix) = uri.strip_prefix(&tmpl.prefix) {
                        return read_resource_reply(
                            id,
                            uri,
                            &tmpl.mime_type,
                            (tmpl.handler)(suffix),
                        );
                    }
                }

                Some(JsonRpcResponse::error(
                    id,
                    -32602,
                    format!("Resource not found: {uri}"),
                ))
            }
            other => Some(JsonRpcResponse::error(
                id,
                -32601,
                format!("Method not found: {other}"),
            )),
        }
    }
}

fn build_initialize_instructions(snap: &RegistrySnapshot) -> String {
    let mut tool_names: Vec<&str> = snap.tools.keys().map(String::as_str).collect();
    tool_names.sort_unstable();

    let mut template_uris: Vec<&str> = snap
        .templates
        .iter()
        .map(|(_, spec)| spec.uri_template.as_str())
        .collect();
    template_uris.sort_unstable();

    let mut out = String::from(
        "Deterministic Vietnamese Luni-Solar calendar & cultural intelligence MCP server anchored to Vietnam UTC+7 (Indochina Time). Delegate all lunar conversions, Vietnam MOLISA holidays/working_weekends, Can Chi & 24 Solar Terms, BaZi/Tử Vi charts, Trạch Nhật scoring, lunar recurrence/.ics exports, and Vietnam retail peaks to these tools instead of estimating mentally.",
    );
    if !tool_names.is_empty() {
        out.push_str("\nActive tools (");
        out.push_str(&tool_names.len().to_string());
        out.push_str("): ");
        out.push_str(&tool_names.join(", "));
        out.push('.');
    }
    if !template_uris.is_empty() {
        out.push_str("\nResource templates: ");
        out.push_str(&template_uris.join(", "));
        out.push('.');
    }
    out
}

fn read_resource_reply(
    id: Option<Value>,
    uri: &str,
    mime_type: &str,
    outcome: Result<Value, crate::contracts::mcp::McpError>,
) -> Option<JsonRpcResponse> {
    match outcome {
        Ok(content) => match serde_json::to_string_pretty(&content) {
            Ok(text) => Some(JsonRpcResponse::success(
                id,
                json!({
                    "contents": [
                        {
                            "uri": uri,
                            "mimeType": mime_type,
                            "text": text
                        }
                    ]
                }),
            )),
            Err(err) => Some(JsonRpcResponse::error(
                id,
                -32603,
                format!("Failed to serialize resource: {err}"),
            )),
        },
        Err(err) => Some(JsonRpcResponse::error(id, err.code(), err.to_string())),
    }
}

/// Cordis plugin providing `dyn McpRegistryService`.
#[derive(Debug, Default)]
pub struct McpCorePlugin;

impl Plugin for McpCorePlugin {
    fn meta(&self) -> PluginMeta {
        PluginMeta::new("mcp_core").provides::<dyn McpRegistryService>()
    }

    fn mount<'a>(&'a self, ctx: Context, _config: Value) -> BoxFuture<'a, Result<(), KernelError>> {
        Box::pin(async move {
            let engine: Arc<dyn McpRegistryService> = Arc::new(McpCoreEngine::new());
            ctx.provide::<dyn McpRegistryService>(engine);
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::mcp::McpError;

    #[test]
    fn test_mcp_core_protocol_dispatch_and_errors() {
        let engine = McpCoreEngine::new();

        let init = engine
            .dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(1)),
                method: "initialize".into(),
                params: None,
            })
            .unwrap();
        assert_eq!(
            init.result.unwrap()["serverInfo"]["version"],
            env!("CARGO_PKG_VERSION")
        );

        let notif = engine.dispatch(JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: None,
            method: "notifications/initialized".into(),
            params: None,
        });
        assert!(notif.is_none());

        let ping = engine
            .dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(2)),
                method: "ping".into(),
                params: None,
            })
            .unwrap();
        assert_eq!(ping.result, Some(json!({})));

        let unknown_method = engine
            .dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(3)),
                method: "does/not/exist".into(),
                params: None,
            })
            .unwrap();
        assert_eq!(unknown_method.error.unwrap().code, -32601);

        let missing_params = engine
            .dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(4)),
                method: "tools/call".into(),
                params: None,
            })
            .unwrap();
        assert_eq!(missing_params.error.unwrap().code, -32602);

        let unknown_tool = engine
            .dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(5)),
                method: "tools/call".into(),
                params: Some(json!({ "name": "missing" })),
            })
            .unwrap();
        assert_eq!(unknown_tool.error.unwrap().code, -32601);
    }

    #[test]
    fn test_mcp_core_sorted_tools_exact_vs_template_and_disposer_guard() {
        let engine = McpCoreEngine::new();

        let d_b = engine.register_tool(McpToolSpec {
            name: "b_tool".into(),
            description: "b".into(),
            input_schema: json!({}),
            handler: Arc::new(|_| Ok(json!({ "v": 1 }))),
        });
        let d_a = engine.register_tool(McpToolSpec {
            name: "a_tool".into(),
            description: "a".into(),
            input_schema: json!({}),
            handler: Arc::new(|_| Ok(json!({ "v": 2 }))),
        });
        let list = engine
            .dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(1)),
                method: "tools/list".into(),
                params: None,
            })
            .unwrap();
        let list_result = list.result.unwrap();
        let names: Vec<&str> = list_result["tools"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|t| t["name"].as_str())
            .collect();
        assert_eq!(names, vec!["a_tool", "b_tool"]);

        // Overwrite a_tool -> invoking stale disposer d_a must NOT delete the newer a_tool.
        let d_a2 = engine.register_tool(McpToolSpec {
            name: "a_tool".into(),
            description: "a2".into(),
            input_schema: json!({}),
            handler: Arc::new(|_| Ok(json!({ "v": 99 }))),
        });
        d_a();
        let call_a = engine
            .dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(2)),
                method: "tools/call".into(),
                params: Some(json!({ "name": "a_tool" })),
            })
            .unwrap();
        assert!(call_a.error.is_none());
        assert!(call_a.result.unwrap()["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("99"));

        // Exact resource takes precedence over template prefix.
        let d_res = engine.register_resource(McpResourceSpec {
            uri: "test://item/exact".into(),
            name: "exact".into(),
            description: "exact".into(),
            mime_type: "application/json".into(),
            handler: Arc::new(|_| Ok(json!({ "from": "exact" }))),
        });
        let d_tmpl = engine.register_resource_template(McpResourceTemplateSpec {
            uri_template: "test://item/{id}".into(),
            name: "tmpl".into(),
            description: "tmpl".into(),
            mime_type: "application/json".into(),
            prefix: "test://item/".into(),
            handler: Arc::new(|s| {
                if s == "bad" {
                    Err(McpError::InvalidParams("bad id".into()))
                } else {
                    Ok(json!({ "from": "tmpl", "id": s }))
                }
            }),
        });

        let r_exact = engine
            .dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(3)),
                method: "resources/read".into(),
                params: Some(json!({ "uri": "test://item/exact" })),
            })
            .unwrap();
        assert!(r_exact.result.unwrap()["contents"][0]["text"]
            .as_str()
            .unwrap()
            .contains("exact"));

        let r_tmpl = engine
            .dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(4)),
                method: "resources/read".into(),
                params: Some(json!({ "uri": "test://item/42" })),
            })
            .unwrap();
        assert!(r_tmpl.result.unwrap()["contents"][0]["text"]
            .as_str()
            .unwrap()
            .contains("42"));

        let r_err = engine
            .dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(5)),
                method: "resources/read".into(),
                params: Some(json!({ "uri": "unknown://x" })),
            })
            .unwrap();
        assert_eq!(r_err.error.unwrap().code, -32602);

        d_b();
        d_a2();
        d_res();
        d_tmpl();
    }

    #[test]
    fn test_initialize_includes_dynamic_live_instructions() {
        let engine = McpCoreEngine::new();
        let init_empty = engine
            .dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(10)),
                method: "initialize".into(),
                params: None,
            })
            .unwrap();
        let inst_empty = init_empty.result.unwrap()["instructions"]
            .as_str()
            .expect("initialize must return instructions string")
            .to_string();
        assert!(inst_empty.contains("UTC+7"));

        let d_tool = engine.register_tool(McpToolSpec {
            name: "demo_tool".into(),
            description: "Demo tool".into(),
            input_schema: json!({ "type": "object" }),
            handler: Arc::new(|_| Ok(json!({ "ok": true }))),
        });
        let d_tmpl = engine.register_resource_template(McpResourceTemplateSpec {
            uri_template: "demo://items/{id}".into(),
            name: "demo".into(),
            description: "demo".into(),
            mime_type: "application/json".into(),
            prefix: "demo://items/".into(),
            handler: Arc::new(|s| Ok(json!({ "id": s }))),
        });

        let init_pop = engine
            .dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(11)),
                method: "initialize".into(),
                params: None,
            })
            .unwrap();
        let inst_pop = init_pop.result.unwrap()["instructions"]
            .as_str()
            .unwrap()
            .to_string();
        assert!(inst_pop.contains("demo_tool"));
        assert!(inst_pop.contains("demo://items/{id}"));

        d_tool();
        let init_after_dispose = engine
            .dispatch(JsonRpcRequest {
                jsonrpc: "2.0".into(),
                id: Some(json!(12)),
                method: "initialize".into(),
                params: None,
            })
            .unwrap();
        let inst_after = init_after_dispose.result.unwrap()["instructions"]
            .as_str()
            .unwrap()
            .to_string();
        assert!(!inst_after.contains("demo_tool"));
        assert!(inst_after.contains("demo://items/{id}"));

        d_tmpl();
    }
}
