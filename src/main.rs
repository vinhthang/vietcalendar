use axum::{routing::get, Router};
use clap::{Parser, Subcommand};
use std::env;

use opentelemetry::global;
use opentelemetry::trace::TracerProvider as _;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use vietcalendar_rs::handlers;
use vietcalendar_rs::mcp;
use vietcalendar_rs::models;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Vietnam Lunar Calendar API",
        version = "0.1.0",
        description = "API for Lunar and Solar dates."
    ),
    paths(
        handlers::home,
        handlers::get_lunar,
        handlers::get_solar_to_lunar,
        handlers::get_lunar_to_solar,
        handlers::check_vietnam_holiday
    ),
    components(schemas(models::DateMonthYear, models::LunarDate, handlers::ErrorResponse))
)]
struct ApiDoc;

use opentelemetry_sdk::trace::TracerProvider;

#[derive(Parser)]
#[command(
    name = "vietcalendar",
    about = "Vietnam Lunar Calendar Service & MCP Server",
    version = "0.1.0"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(short, long, env = "PORT")]
    port: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the Axum HTTP REST API Web Server (default)
    Serve {
        #[arg(short, long, env = "PORT")]
        port: Option<String>,
    },
    /// Start the Model Context Protocol (MCP) Stdio Server [Alpha]
    Mcp,
}

fn init_telemetry() {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "info,vietcalendar_rs=debug,tower_http=debug".into());

    let exporter = match opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .build()
    {
        Ok(e) => Some(e),
        Err(e) => {
            eprintln!("Failed to build otlp exporter: {:?}", e);
            None
        }
    };

    if let Some(exporter) = exporter {
        let provider = TracerProvider::builder()
            .with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)
            .build();
        global::set_tracer_provider(provider.clone());
        let tracer = provider.tracer("vietcalendar-rs");

        let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

        tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer())
            .with(telemetry)
            .init();
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer())
            .init();
    }
}

async fn run_http_server(port_opt: Option<String>) {
    init_telemetry();

    let port = port_opt
        .or_else(|| env::var("PORT").ok())
        .or_else(|| env::var("HTTP_PORT").ok())
        .unwrap_or_else(|| "8080".to_string());

    let addr = format!("0.0.0.0:{}", port);

    let app = Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .route("/", get(handlers::home))
        .route("/lunar", get(handlers::get_lunar))
        .route(
            "/convert/solar-to-lunar/{date}",
            get(handlers::get_solar_to_lunar),
        )
        .route(
            "/convert/lunar-to-solar/{date}",
            get(handlers::get_lunar_to_solar),
        )
        .route("/vietnam-holiday", get(handlers::check_vietnam_holiday))
        .layer(TraceLayer::new_for_http());

    tracing::info!("Listening on http://{}", addr);
    tracing::info!("Swagger UI at http://{}/swagger-ui", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Mcp) => {
            if let Err(e) = mcp::run_stdio_server().await {
                eprintln!("MCP server error: {}", e);
            }
        }
        Some(Commands::Serve { port }) => {
            run_http_server(port.or(cli.port)).await;
        }
        None => {
            run_http_server(cli.port).await;
        }
    }
}
