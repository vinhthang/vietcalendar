use axum::{routing::get, Router};
use std::env;

use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use opentelemetry::global;
use opentelemetry::trace::TracerProvider as _;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use vietcalendar_rs::handlers;
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
    components(
        schemas(models::DateMonthYear, models::LunarDate, handlers::ErrorResponse)
    )
)]
struct ApiDoc;

use opentelemetry_sdk::trace::TracerProvider;

fn init_telemetry() {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "info,vietcalendar_rs=debug,tower_http=debug".into());

    let exporter = match opentelemetry_otlp::SpanExporter::builder().with_tonic().build() {
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

#[tokio::main]
async fn main() {
    init_telemetry();

    let port = env::var("PORT")
        .or_else(|_| env::var("HTTP_PORT"))
        .unwrap_or_else(|_| "8080".to_string());
    
    let addr = format!("0.0.0.0:{}", port);
    
    let app = Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .route("/", get(handlers::home))
        .route("/lunar", get(handlers::get_lunar))
        .route("/convert/solar-to-lunar/{date}", get(handlers::get_solar_to_lunar))
        .route("/convert/lunar-to-solar/{date}", get(handlers::get_lunar_to_solar))
        .route("/vietnam-holiday", get(handlers::check_vietnam_holiday))
        .layer(TraceLayer::new_for_http());


    tracing::info!("Listening on http://{}", addr);
    tracing::info!("Swagger UI at http://{}/swagger-ui", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
