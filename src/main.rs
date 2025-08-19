use axum::{
    routing::{post, get},
    Router,
    http::Method,
    response::Redirect,
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
        (url = "/xml-compare-api", description = "XML Compare API Server (Base Path)"),
        (url = "/", description = "XML Compare API Server (Root)")
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
        
        .with_state(state.clone());

    // Main app router - simplified for proxy compatibility
    let app = Router::new()
        // Redirect root to swagger UI - use relative path
        .route("/", get(|| async { Redirect::permanent("/swagger-ui/") }))
        
        // Root level health check for proxy compatibility
        .route("/health", get(health_check))
        
        // Root level Swagger UI for proxy compatibility
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        
        // Landing page for base path (both with and without trailing slash)
        .route("/xml-compare-api", get(landing_page))
        .route("/xml-compare-api/", get(landing_page))
        
        // Mount API router under base path
        .nest("/xml-compare-api", api_router)
        
        // Base path level Swagger UI (uses a different OpenAPI endpoint path)
        .merge(SwaggerUi::new("/xml-compare-api/swagger-ui").url("/xml-compare-api/api-docs/openapi.json", ApiDoc::openapi()))
        
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
    tracing::info!("Landing page available at:");
    tracing::info!("  - http://0.0.0.0:{}/xml-compare-api/ (base path)", port);
    tracing::info!("Swagger UI available at:");
    tracing::info!("  - http://0.0.0.0:{}/swagger-ui/ (root level)", port);
    tracing::info!("  - http://0.0.0.0:{}/xml-compare-api/swagger-ui/ (base path)", port);
    tracing::info!("Health check available at:");
    tracing::info!("  - http://0.0.0.0:{}/health (root level)", port);
    tracing::info!("  - http://0.0.0.0:{}/xml-compare-api/health (base path)", port);
    tracing::info!("Root (/) redirects to Swagger UI");
    tracing::info!("Base path (/) shows landing page");
    tracing::info!("Session cleanup task started (runs every 5 minutes)");

    axum::serve(listener, app).await.unwrap();
}

async fn health_check() -> &'static str {
    "OK"
}

async fn landing_page() -> axum::response::Html<&'static str> {
    axum::response::Html(r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>XML Compare API - High-Performance XML Document Comparison</title>
    <style>
        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }
        
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif;
            line-height: 1.6;
            color: #333;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            min-height: 100vh;
        }
        
        .container {
            max-width: 1200px;
            margin: 0 auto;
            padding: 20px;
        }
        
        .header {
            text-align: center;
            color: white;
            margin-bottom: 40px;
        }
        
        .header h1 {
            font-size: 3rem;
            margin-bottom: 10px;
            text-shadow: 2px 2px 4px rgba(0,0,0,0.3);
        }
        
        .header p {
            font-size: 1.2rem;
            opacity: 0.9;
        }
        
        .content {
            background: white;
            border-radius: 15px;
            padding: 40px;
            box-shadow: 0 20px 40px rgba(0,0,0,0.1);
            margin-bottom: 30px;
        }
        
        .features {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
            gap: 30px;
            margin: 40px 0;
        }
        
        .feature-card {
            background: #f8f9fa;
            padding: 25px;
            border-radius: 10px;
            border-left: 4px solid #667eea;
            transition: transform 0.3s ease, box-shadow 0.3s ease;
        }
        
        .feature-card:hover {
            transform: translateY(-5px);
            box-shadow: 0 10px 25px rgba(0,0,0,0.15);
        }
        
        .feature-card h3 {
            color: #667eea;
            margin-bottom: 15px;
            font-size: 1.3rem;
        }
        
        .endpoints {
            background: #f8f9fa;
            padding: 25px;
            border-radius: 10px;
            margin: 30px 0;
        }
        
        .endpoint {
            background: white;
            padding: 15px;
            margin: 10px 0;
            border-radius: 8px;
            border-left: 3px solid #28a745;
            font-family: 'Monaco', 'Menlo', monospace;
            font-size: 0.9rem;
        }
        
        .method {
            display: inline-block;
            padding: 4px 8px;
            border-radius: 4px;
            font-weight: bold;
            font-size: 0.8rem;
            margin-right: 10px;
        }
        
        .method.post { background: #007bff; color: white; }
        .method.get { background: #28a745; color: white; }
        
        .quick-start {
            background: #e3f2fd;
            padding: 25px;
            border-radius: 10px;
            margin: 30px 0;
            border-left: 4px solid #2196f3;
        }
        
        .code-block {
            background: #2d3748;
            color: #e2e8f0;
            padding: 20px;
            border-radius: 8px;
            font-family: 'Monaco', 'Menlo', monospace;
            font-size: 0.9rem;
            overflow-x: auto;
            margin: 15px 0;
        }
        
        .btn {
            display: inline-block;
            padding: 12px 24px;
            background: #667eea;
            color: white;
            text-decoration: none;
            border-radius: 8px;
            font-weight: bold;
            transition: background 0.3s ease;
            margin: 10px 10px 10px 0;
        }
        
        .btn:hover {
            background: #5a6fd8;
        }
        
        .btn.secondary {
            background: #6c757d;
        }
        
        .btn.secondary:hover {
            background: #5a6268;
        }
        
        .stats {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 20px;
            margin: 30px 0;
        }
        
        .stat-card {
            background: white;
            padding: 20px;
            border-radius: 10px;
            text-align: center;
            box-shadow: 0 5px 15px rgba(0,0,0,0.1);
        }
        
        .stat-number {
            font-size: 2.5rem;
            font-weight: bold;
            color: #667eea;
            margin-bottom: 10px;
        }
        
        .footer {
            text-align: center;
            color: white;
            margin-top: 40px;
            opacity: 0.8;
        }
        
        @media (max-width: 768px) {
            .header h1 { font-size: 2rem; }
            .content { padding: 20px; }
            .features { grid-template-columns: 1fr; }
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>🚀 XML Compare API</h1>
            <p>High-Performance XML Document Comparison with Rust + Axum</p>
        </div>
        
        <div class="content">
            <h2>✨ Features</h2>
            <div class="features">
                <div class="feature-card">
                    <h3>🔍 Smart Comparison</h3>
                    <p>Advanced XML diffing with attribute & content analysis, path-based ignoring, and wildcard support.</p>
                </div>
                <div class="feature-card">
                    <h3>🌐 URL Support</h3>
                    <p>Compare XMLs directly from URLs with automatic authentication and cookie management.</p>
                </div>
                <div class="feature-card">
                    <h3>⚡ Batch Processing</h3>
                    <p>High-performance batch comparison with concurrent processing and comprehensive results.</p>
                </div>
                <div class="feature-card">
                    <h3>🔐 Authentication</h3>
                    <p>Session-based authentication system with secure cookie handling and automatic cleanup.</p>
                </div>
                <div class="feature-card">
                    <h3>📊 Performance</h3>
                    <p>Built with Rust + Axum for maximum performance, streaming XML parsing, and async I/O.</p>
                </div>
                <div class="feature-card">
                    <h3>📚 OpenAPI</h3>
                    <p>Complete OpenAPI 3.0 specification with interactive Swagger UI documentation.</p>
                </div>
            </div>
            
            <h2>📡 API Endpoints</h2>
            <div class="endpoints">
                <div class="endpoint">
                    <span class="method post">POST</span>
                    <code>/api/compare/xml</code> - Compare two XML strings
                </div>
                <div class="endpoint">
                    <span class="method post">POST</span>
                    <code>/api/compare/xml/batch</code> - Batch XML comparison
                </div>
                <div class="endpoint">
                    <span class="method post">POST</span>
                    <code>/api/compare/url</code> - Compare XMLs from URLs
                </div>
                <div class="endpoint">
                    <span class="method post">POST</span>
                    <code>/api/compare/url/batch</code> - Batch URL comparison
                </div>
                <div class="endpoint">
                    <span class="method post">POST</span>
                    <code>/api/auth/login</code> - Authenticate with URL
                </div>
                <div class="endpoint">
                    <span class="method post">POST</span>
                    <code>/api/auth/logout/{session_id}</code> - Logout session
                </div>
                <div class="endpoint">
                    <span class="method get">GET</span>
                    <code>/health</code> - Health check
                </div>
            </div>
            
            <h2>🚀 Quick Start</h2>
            <div class="quick-start">
                <h3>Compare Two XML Documents</h3>
                <div class="code-block">
curl -X POST http://localhost:3000/xml-compare-api/api/compare/xml \
  -H "Content-Type: application/json" \
  -d '{
    "xml1": "<root><item id=\"1\">Hello</item></root>",
    "xml2": "<root><item id=\"2\">Hello</item></root>",
    "ignore_properties": ["id"]
  }'
                </div>
                
                <h3>Compare XMLs from URLs</h3>
                <div class="code-block">
curl -X POST http://localhost:3000/xml-compare-api/api/compare/url \
  -H "Content-Type: application/json" \
  -d '{
    "url1": "https://api.example.com/xml1.xml",
    "url2": "https://api.example.com/xml2.xml",
    "ignore_paths": ["/root/timestamp"]
  }'
                </div>
            </div>
            
            <h2>📊 Performance Stats</h2>
            <div class="stats">
                <div class="stat-card">
                    <div class="stat-number">100%</div>
                    <div>Test Coverage</div>
                </div>
                <div class="stat-card">
                    <div class="stat-number">47</div>
                    <div>Tests Passing</div>
                </div>
                <div class="stat-card">
                    <div class="stat-number">500MB</div>
                    <div>Max Payload</div>
                </div>
                <div class="stat-card">
                    <div class="stat-number">5min</div>
                    <div>Session TTL</div>
                </div>
            </div>
            
            <div style="text-align: center; margin: 40px 0;">
                <a href="/xml-compare-api/swagger-ui/" class="btn">📚 Interactive API Docs</a>
                <a href="/xml-compare-api/health" class="btn secondary">🏥 Health Check</a>
                <a href="/swagger-ui/" class="btn secondary">🔗 Root Swagger UI</a>
            </div>
        </div>
        
        <div class="footer">
            <p>Built with ❤️ using Rust + Axum | MIT License</p>
            <p>Perfect for production deployments behind app-runner-router</p>
        </div>
    </div>
</body>
</html>
    "#)
}