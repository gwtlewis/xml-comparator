use axum::{
    routing::{post, get},
    Router,
    http::Method,
    response::Redirect,
    Json,
    extract::DefaultBodyLimit,
};
use tower_http::cors::{CorsLayer, Any};
use std::sync::Arc;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

mod models;
mod services;
mod handlers;
mod utils;

use handlers::{comparison_handlers, auth_handlers};
use handlers::comparison_handlers::AppStateInner;
use services::{XmlComparisonService, HttpClientService, AuthService};

#[derive(OpenApi)]
#[openapi(
    paths(
        comparison_handlers::compare_xmls,
        comparison_handlers::compare_urls,
        comparison_handlers::compare_xmls_batch,
        comparison_handlers::compare_urls_batch,
        auth_handlers::login,
        auth_handlers::logout
    ),
    components(
        schemas(
            models::XmlComparisonRequest,
            models::XmlComparisonResponse,
            models::XmlDiff,
            models::DiffType,
            models::UrlComparisonRequest,
            models::AuthCredentials,
            models::BatchXmlComparisonRequest,
            models::BatchUrlComparisonRequest,
            models::BatchComparisonResponse,
            models::LoginRequest,
            models::LoginResponse,
            models::AppError
        )
    ),
    tags(
        (name = "XML Comparison", description = "XML comparison endpoints"),
        (name = "URL Comparison", description = "URL-based XML comparison endpoints"),
        (name = "Batch Comparison", description = "Batch XML comparison endpoints"),
        (name = "Authentication", description = "Authentication endpoints")
    ),
    servers(
        (url = "/xml-compare-api", description = "XML Compare API Server")
    ),
    info(
        title = "XML Comparison API",
        version = "1.0.0",
        description = "A REST API for comparing XML documents with support for URL-based comparison, batch processing, and authentication."
    )
)]
struct ApiDoc;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Get port from environment variable or default to 3000
    let port = std::env::var("APP_PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()
        .unwrap_or(3000);

    // Create services
    let xml_service = XmlComparisonService::new();
    let http_client = Arc::new(HttpClientService::new());
    let auth_service = Arc::new(AuthService::new(http_client.clone()));

    // Create app state
    let state = Arc::new(AppStateInner {
        xml_service,
        http_client,
        auth_service,
    });

    // Configure CORS
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_origin(Any);

    // Create API router with base path
    let api_router = Router::new()
        // XML comparison endpoints
        .route("/api/compare/xml", post(comparison_handlers::compare_xmls))
        .route("/api/compare/xml/batch", post(comparison_handlers::compare_xmls_batch))
        
        // URL comparison endpoints
        .route("/api/compare/url", post(comparison_handlers::compare_urls))
        .route("/api/compare/url/batch", post(comparison_handlers::compare_urls_batch))
        
        // Authentication endpoints
        .route("/api/auth/login", post(auth_handlers::login))
        .route("/api/auth/logout/:session_id", post(auth_handlers::logout))
        
        // Health check
        .route("/health", get(health_check))
        
        // Manual OpenAPI JSON route
        .route("/api-docs/openapi.json", get(|| async {
            Json(ApiDoc::openapi())
        }))
        
        // Swagger UI with correct absolute path
        .merge(SwaggerUi::new("/swagger-ui").url("/xml-compare-api/api-docs/openapi.json", ApiDoc::openapi()))
        
        .with_state(state.clone());

    // Main app router with base path and root redirect
    let app = Router::new()
        // Redirect root to swagger UI
        .route("/", get(|| async { Redirect::permanent("/xml-compare-api/swagger-ui/") }))
        
        // Mount API router under base path
        .nest("/xml-compare-api", api_router)
        
        // Configure body limits (500MB for large batch operations)
        .layer(DefaultBodyLimit::max(500 * 1024 * 1024))
        .layer(cors);

        // Start background session cleanup task
    let auth_service_cleanup = state.auth_service.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(300)); // Clean up every 5 minutes
        loop {
            interval.tick().await;
            auth_service_cleanup.cleanup_expired_sessions().await;
            tracing::debug!("Cleaned up expired sessions");
        }
    });

    // Start server
    let bind_address = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&bind_address).await.unwrap();

    tracing::info!("Server running on http://0.0.0.0:{}", port);
    tracing::info!("Swagger UI available at http://0.0.0.0:{}/xml-compare-api/swagger-ui/", port);
    tracing::info!("Health check available at http://0.0.0.0:{}/xml-compare-api/health", port);
    tracing::info!("Root (/) redirects to Swagger UI");
    tracing::info!("Session cleanup task started (runs every 5 minutes)");

    axum::serve(listener, app).await.unwrap();
}

async fn health_check() -> &'static str {
    "OK"
}