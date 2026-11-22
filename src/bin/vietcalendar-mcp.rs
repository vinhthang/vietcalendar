//! Dedicated `vietcalendar-mcp` binary booting the Cordis `mcp-stdio` profile.

use std::process::ExitCode;

use vietcalendar_rs::plugins::mcp_stdio::StdioExitSignal;
use vietcalendar_rs::profiles::{boot_profile, resolve_config_dir};

#[tokio::main]
async fn main() -> ExitCode {
    let config_dir = resolve_config_dir(None);
    let kernel = match boot_profile(&config_dir, "mcp-stdio", None, None).await {
        Ok(k) => k,
        Err(err) => {
            eprintln!("MCP server startup error: {err}");
            return ExitCode::FAILURE;
        }
    };

    if let Some(exit_sig) = kernel.inject::<StdioExitSignal>() {
        tokio::select! {
            _ = exit_sig.notified() => {}
            _ = tokio::signal::ctrl_c() => {}
        }
    } else {
        let _ = tokio::signal::ctrl_c().await;
    }

    kernel.shutdown().await;
    ExitCode::SUCCESS
}
