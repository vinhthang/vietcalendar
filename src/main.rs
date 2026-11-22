//! `vietcalendar` dual-mode CLI binary booting Cordis profiles (`serve` or `mcp`).

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use vietcalendar_rs::kernel::Kernel;
use vietcalendar_rs::plugins::mcp_stdio::StdioExitSignal;
use vietcalendar_rs::profiles::{boot_profile, resolve_config_dir};

#[derive(Parser)]
#[command(
    name = "vietcalendar",
    about = "Vietnam Lunar Calendar Service & MCP Server",
    version = env!("CARGO_PKG_VERSION")
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Optional HTTP port override (falls back to YAML port, $PORT, $HTTP_PORT, or 8080).
    #[arg(short, long)]
    port: Option<u16>,

    /// Path to the YAML configuration directory (default: $VIETCALENDAR_CONFIG_DIR or ./config).
    #[arg(long, env = "VIETCALENDAR_CONFIG_DIR")]
    config_dir: Option<PathBuf>,

    /// Cordis profile name to boot from cordis.yml.
    #[arg(long)]
    profile: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the non-blocking HTTP REST API + Remote MCP SSE server (default)
    Serve {
        /// Optional HTTP port override (overrides YAML port and env vars).
        #[arg(short, long)]
        port: Option<u16>,
        /// Path to the YAML configuration directory.
        #[arg(long, env = "VIETCALENDAR_CONFIG_DIR")]
        config_dir: Option<PathBuf>,
        /// Cordis profile name (default: "serve").
        #[arg(long)]
        profile: Option<String>,
    },
    /// Start the Model Context Protocol (MCP) Stdio Server
    Mcp {
        /// Path to the YAML configuration directory.
        #[arg(long, env = "VIETCALENDAR_CONFIG_DIR")]
        config_dir: Option<PathBuf>,
        /// Cordis profile name (default: "mcp-stdio").
        #[arg(long)]
        profile: Option<String>,
    },
}

async fn wait_for_shutdown_signal() {
    #[cfg(unix)]
    {
        let mut sigterm =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).ok();
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = async {
                if let Some(ref mut s) = sigterm {
                    s.recv().await;
                } else {
                    std::future::pending::<()>().await;
                }
            } => {}
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

async fn run_serve(config_dir: PathBuf, profile: String, port: Option<u16>) -> Result<(), String> {
    let kernel: Kernel = boot_profile(&config_dir, &profile, port, None)
        .await
        .map_err(|e| format!("failed to boot profile '{profile}': {e}"))?;

    wait_for_shutdown_signal().await;
    kernel.shutdown().await;
    Ok(())
}

async fn run_mcp(config_dir: PathBuf, profile: String) -> Result<(), String> {
    let kernel: Kernel = boot_profile(&config_dir, &profile, None, None)
        .await
        .map_err(|e| format!("failed to boot profile '{profile}': {e}"))?;

    if let Some(exit_sig) = kernel.inject::<StdioExitSignal>() {
        tokio::select! {
            _ = exit_sig.notified() => {}
            _ = wait_for_shutdown_signal() => {}
        }
    } else {
        wait_for_shutdown_signal().await;
    }

    kernel.shutdown().await;
    Ok(())
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    let result = match cli.command {
        Some(Commands::Mcp {
            config_dir,
            profile,
        }) => {
            let dir = resolve_config_dir(config_dir.or(cli.config_dir));
            let prof = profile
                .or(cli.profile)
                .unwrap_or_else(|| "mcp-stdio".to_string());
            run_mcp(dir, prof).await
        }
        Some(Commands::Serve {
            port,
            config_dir,
            profile,
        }) => {
            let dir = resolve_config_dir(config_dir.or(cli.config_dir));
            let prof = profile
                .or(cli.profile)
                .unwrap_or_else(|| "serve".to_string());
            run_serve(dir, prof, port.or(cli.port)).await
        }
        None => {
            let dir = resolve_config_dir(cli.config_dir);
            let prof = cli.profile.unwrap_or_else(|| "serve".to_string());
            run_serve(dir, prof, cli.port).await
        }
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("vietcalendar error: {err}");
            ExitCode::FAILURE
        }
    }
}
