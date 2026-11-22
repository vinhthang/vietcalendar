//! `McpStdioPlugin`: non-blocking cancellation-aware JSON-RPC 2.0 `stdio` transport.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use serde_json::Value;
use tokio::io::{self, AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::sync::Notify;
use tracing::{error, info};

use crate::contracts::mcp::{JsonRpcRequest, JsonRpcResponse, McpRegistryService};
use crate::kernel::context::Context;
use crate::kernel::effect::BoxFuture;
use crate::kernel::error::KernelError;
use crate::kernel::fiber::{Plugin, PluginMeta};

/// Signal notified when the `mcp_stdio` transport reaches `stdin` EOF or exits.
#[derive(Debug, Default)]
pub struct StdioExitSignal {
    exited: AtomicBool,
    notify: Notify,
}

impl StdioExitSignal {
    /// Awaits completion of the `stdio` loop without missing early exits.
    pub async fn notified(&self) {
        loop {
            if self.exited.load(Ordering::Acquire) {
                return;
            }
            let fut = self.notify.notified();
            tokio::pin!(fut);
            fut.as_mut().enable();
            if self.exited.load(Ordering::Acquire) {
                return;
            }
            fut.await;
        }
    }

    fn fire(&self) {
        self.exited.store(true, Ordering::Release);
        self.notify.notify_waiters();
    }
}

/// Runs a cancellation-aware line-delimited JSON-RPC 2.0 loop over arbitrary async streams.
///
/// # Errors
/// Returns `io::Error` if reading from `reader` or writing to `writer` fails.
pub async fn run_stdio_with_io<R, W>(
    registry: Arc<dyn McpRegistryService>,
    reader: R,
    mut writer: W,
    token: tokio_util::sync::CancellationToken,
) -> io::Result<()>
where
    R: AsyncBufRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let mut lines = reader.lines();

    loop {
        let maybe_line = tokio::select! {
            biased;
            _ = token.cancelled() => {
                info!("MCP stdio transport cancelled");
                break;
            }
            res = lines.next_line() => res?,
        };

        let Some(line) = maybe_line else {
            break;
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let response_opt = match serde_json::from_str::<JsonRpcRequest>(trimmed) {
            Ok(req) => registry.dispatch(req),
            Err(_) => Some(JsonRpcResponse::error(
                None,
                -32700,
                "Parse error: Invalid JSON",
            )),
        };

        if let Some(resp) = response_opt {
            let line_json = serde_json::to_string(&resp).unwrap_or_else(|e| {
                serde_json::to_string(&JsonRpcResponse::error(
                    resp.id.clone().or(Some(Value::Null)),
                    -32603,
                    format!("Serialization error: {e}"),
                ))
                .unwrap_or_else(|_| {
                    r#"{"jsonrpc":"2.0","id":null,"error":{"code":-32603,"message":"internal error"}}"#
                        .to_string()
                })
            });
            writer.write_all(line_json.as_bytes()).await?;
            writer.write_all(b"\n").await?;
            writer.flush().await?;
        }
    }

    Ok(())
}

/// Runs a cancellation-aware line-delimited JSON-RPC 2.0 `stdio` loop.
///
/// # Errors
/// Returns `io::Error` if reading from `stdin` or writing to `stdout` fails.
pub async fn run_stdio_loop(
    registry: Arc<dyn McpRegistryService>,
    token: tokio_util::sync::CancellationToken,
) -> io::Result<()> {
    let reader = BufReader::new(io::stdin());
    let writer = io::stdout();
    run_stdio_with_io(registry, reader, writer, token).await
}

/// Cordis plugin launching the MCP JSON-RPC `stdio` transport bound to fiber lifetime.
#[derive(Debug, Default)]
pub struct McpStdioPlugin;

impl Plugin for McpStdioPlugin {
    fn meta(&self) -> PluginMeta {
        PluginMeta::new("mcp_stdio")
            .requires::<dyn McpRegistryService>()
            .provides::<StdioExitSignal>()
    }

    fn mount<'a>(&'a self, ctx: Context, _config: Value) -> BoxFuture<'a, Result<(), KernelError>> {
        Box::pin(async move {
            let registry = ctx.inject::<dyn McpRegistryService>()?;
            let exit_signal = Arc::new(StdioExitSignal::default());
            ctx.provide(Arc::clone(&exit_signal));

            ctx.spawn("mcp_stdio_loop", move |token| async move {
                if let Err(err) = run_stdio_loop(registry, token).await {
                    error!(error = %err, "MCP stdio transport terminated with error");
                }
                exit_signal.fire();
            });
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::mcp_core::McpCoreEngine;
    use std::io::Cursor;
    use std::time::Duration;
    use tokio_util::sync::CancellationToken;

    #[tokio::test]
    async fn test_stdio_exit_signal_does_not_miss_early_fire() {
        let sig = StdioExitSignal::default();
        sig.fire();
        tokio::time::timeout(Duration::from_millis(200), sig.notified())
            .await
            .expect("StdioExitSignal::notified missed early fire");
    }

    #[tokio::test]
    async fn test_stdio_exit_signal_wakes_concurrent_waiter() {
        let sig = Arc::new(StdioExitSignal::default());
        assert!(
            tokio::time::timeout(Duration::from_millis(25), sig.notified())
                .await
                .is_err(),
            "StdioExitSignal::notified must wait until fire() is called"
        );
        let sig_waker = Arc::clone(&sig);
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            sig_waker.fire();
        });
        tokio::time::timeout(Duration::from_millis(500), sig.notified())
            .await
            .expect("StdioExitSignal::notified timed out waiting for wakeup");
    }

    #[tokio::test]
    async fn test_run_stdio_with_io_handles_ping_notification_and_parse_error() {
        let registry: Arc<dyn McpRegistryService> = Arc::new(McpCoreEngine::new());
        let input = concat!(
            "\n",
            "{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n",
            "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}\n",
            "{invalid json}\n",
        );
        let reader = BufReader::new(Cursor::new(input.as_bytes()));
        let mut output: Vec<u8> = Vec::new();

        run_stdio_with_io(registry, reader, &mut output, CancellationToken::new())
            .await
            .unwrap();

        let text = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2);

        let resp1: JsonRpcResponse = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(resp1.id, Some(Value::from(1)));
        assert!(resp1.result.is_some());
        assert!(resp1.error.is_none());

        let resp2: JsonRpcResponse = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(resp2.error.unwrap().code, -32700);
    }
}
