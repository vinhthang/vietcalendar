//! Service contract (`McpRegistryService`) and JSON-RPC 2.0 wire types for MCP plugins.

use std::fmt;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Incoming JSON-RPC 2.0 request envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    /// JSON-RPC protocol version (`"2.0"`).
    pub jsonrpc: String,
    /// Optional request identifier (`None` for notifications).
    pub id: Option<Value>,
    /// Target RPC method name.
    pub method: String,
    /// Optional structured parameters.
    #[serde(default)]
    pub params: Option<Value>,
}

/// Outgoing JSON-RPC 2.0 response envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    /// JSON-RPC protocol version (`"2.0"`).
    pub jsonrpc: String,
    /// Matching request identifier.
    pub id: Option<Value>,
    /// Success result payload.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    /// Error payload when the request failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// Structured JSON-RPC 2.0 error object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    /// Standard JSON-RPC error code (`-32700`, `-32601`, `-32602`, `-32603`).
    pub code: i64,
    /// Human-readable error message.
    pub message: String,
    /// Optional diagnostic error metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcResponse {
    /// Constructs a successful JSON-RPC 2.0 response.
    pub fn success(id: Option<Value>, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    /// Constructs a JSON-RPC 2.0 error response.
    pub fn error(id: Option<Value>, code: i64, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.into(),
                data: None,
            }),
        }
    }
}

/// Errors emitted by MCP tool and resource handlers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpError {
    /// Malformed JSON payload (`-32700`).
    ParseError(String),
    /// Unknown JSON-RPC method (`-32601`).
    MethodNotFound(String),
    /// Missing or invalid tool/resource parameters, including unknown tool name (`-32602`).
    InvalidParams(String),
    /// Internal server error (`-32603`).
    Internal(String),
}

impl McpError {
    /// Returns the JSON-RPC error code corresponding to this variant.
    pub fn code(&self) -> i64 {
        match self {
            Self::ParseError(_) => -32700,
            Self::MethodNotFound(_) => -32601,
            Self::InvalidParams(_) => -32602,
            Self::Internal(_) => -32603,
        }
    }
}

impl fmt::Display for McpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ParseError(m)
            | Self::MethodNotFound(m)
            | Self::InvalidParams(m)
            | Self::Internal(m) => f.write_str(m),
        }
    }
}

impl std::error::Error for McpError {}

/// Function handler executing an MCP tool call with JSON arguments.
pub type McpToolHandler = Arc<dyn Fn(Value) -> Result<Value, McpError> + Send + Sync>;

/// Function handler reading an MCP resource URI.
pub type McpResourceHandler = Arc<dyn Fn(&str) -> Result<Value, McpError> + Send + Sync>;

/// Disposer handle removing a registered MCP tool, resource, or template on unwind.
pub type McpDisposer = Box<dyn FnOnce() + Send>;

/// Tool definition registered into `McpRegistryService`.
#[derive(Clone)]
pub struct McpToolSpec {
    /// Unique tool identifier.
    pub name: String,
    /// Human-readable description shown to MCP clients.
    pub description: String,
    /// JSON Schema describing expected `arguments`.
    pub input_schema: Value,
    /// Execution handler returning structured JSON content.
    pub handler: McpToolHandler,
}

/// Concrete resource entry registered into `McpRegistryService`.
#[derive(Clone)]
pub struct McpResourceSpec {
    /// Exact resource URI (e.g., `"calendar://today"`).
    pub uri: String,
    /// Display name.
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// MIME media type (typically `"application/json"`).
    pub mime_type: String,
    /// Reader returning JSON content for this URI.
    pub handler: McpResourceHandler,
}

/// URI-template resource pattern registered into `McpRegistryService`.
#[derive(Clone)]
pub struct McpResourceTemplateSpec {
    /// RFC 6570 URI template advertised via `resources/templates/list`.
    pub uri_template: String,
    /// Display name.
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// MIME media type.
    pub mime_type: String,
    /// Prefix matched against incoming `resources/read` URIs (e.g., `"calendar://holidays/"`).
    pub prefix: String,
    /// Reader invoked with the suffix following `prefix`.
    pub handler: McpResourceHandler,
}

/// Pluggable MCP JSON-RPC dispatcher capability provided by `McpCorePlugin`.
pub trait McpRegistryService: Send + Sync + 'static {
    /// Registers an MCP tool, returning a LIFO disposer handle.
    fn register_tool(&self, spec: McpToolSpec) -> McpDisposer;

    /// Registers an MCP resource, returning a LIFO disposer handle.
    fn register_resource(&self, spec: McpResourceSpec) -> McpDisposer;

    /// Registers an MCP URI resource template, returning a LIFO disposer handle.
    fn register_resource_template(&self, spec: McpResourceTemplateSpec) -> McpDisposer;

    /// Dispatches a JSON-RPC 2.0 request against wait-free registry snapshots.
    fn dispatch(&self, req: JsonRpcRequest) -> Option<JsonRpcResponse>;
}
