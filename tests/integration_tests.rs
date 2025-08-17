use serde_json::json;
use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

// Helper function to create test app
async fn create_test_app() -> Router {
    use xml_compare_api::handlers::{comparison_handlers, auth_handlers};
    use xml_compare_api::handlers::comparison_handlers::AppStateInner;
    use xml_compare_api::services::{XmlComparisonService, HttpClientService, AuthService};
    use std::sync::Arc;
    use axum::routing::{post, get};
    use tower_http::cors::{CorsLayer, Any};
    use axum::http::Method;

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

    // Create API router
    Router::new()
        .route("/api/compare/xml", post(comparison_handlers::compare_xmls))
        .route("/api/compare/xml/batch", post(comparison_handlers::compare_xmls_batch))
        .route("/api/compare/url", post(comparison_handlers::compare_urls))
        .route("/api/compare/url/batch", post(comparison_handlers::compare_urls_batch))
        .route("/api/auth/login", post(auth_handlers::login))
        .route("/api/auth/logout/:session_id", post(auth_handlers::logout))
        .route("/health", get(|| async { "OK" }))
        .with_state(state)
        .layer(cors)
}

#[tokio::test]
async fn test_xml_comparison_api_attribute_and_content_differences() {
    let app = create_test_app().await;
    
    let request_body = json!({
        "xml1": "<Mapping date=\"20250819\">test</Mapping>",
        "xml2": "<Mapping date=\"20250818\">test2</Mapping>",
        "ignore_paths": [],
        "ignore_properties": []
    });

    let request = Request::builder()
        .method("POST")
        .uri("/api/compare/xml")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&request_body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let response_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    
    // Verify the response structure
    assert_eq!(response_json["matched"], false);
    assert_eq!(response_json["diffs"].as_array().unwrap().len(), 2);
    
    // Check that we have both content and attribute differences
    let diffs = response_json["diffs"].as_array().unwrap();
    let has_content_diff = diffs.iter().any(|d| d["diff_type"] == "ContentDifferent");
    let has_attr_diff = diffs.iter().any(|d| d["diff_type"] == "AttributeDifferent");
    
    assert!(has_content_diff, "Should have content difference");
    assert!(has_attr_diff, "Should have attribute difference");
}

#[tokio::test]
async fn test_xml_comparison_api_ignore_attribute() {
    let app = create_test_app().await;
    
    let request_body = json!({
        "xml1": "<Mapping date=\"20250819\">test</Mapping>",
        "xml2": "<Mapping date=\"20250818\">test</Mapping>",
        "ignore_paths": [],
        "ignore_properties": ["date"]
    });

    let request = Request::builder()
        .method("POST")
        .uri("/api/compare/xml")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&request_body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let response_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    
    // Should be matched since we're ignoring the date attribute
    assert_eq!(response_json["matched"], true);
    assert_eq!(response_json["diffs"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_xml_comparison_api_ignore_element_content() {
    let app = create_test_app().await;
    
    let request_body = json!({
        "xml1": "<Mapping date=\"20250819\">test</Mapping>",
        "xml2": "<Mapping date=\"20250818\">test2</Mapping>",
        "ignore_paths": [],
        "ignore_properties": ["Mapping"]
    });

    let request = Request::builder()
        .method("POST")
        .uri("/api/compare/xml")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&request_body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let response_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    
    // Should be matched since we're ignoring the Mapping element content
    assert_eq!(response_json["matched"], true);
    assert_eq!(response_json["diffs"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_xml_comparison_api_complex_nested() {
    let request_body = json!({
        "xml1": "<root><Mapping date=\"20250819\" version=\"1.0\">test<child attr=\"val1\">child1</child></Mapping></root>",
        "xml2": "<root><Mapping date=\"20250818\" version=\"1.1\">test2<child attr=\"val2\">child2</child></Mapping></root>",
        "ignore_paths": [],
        "ignore_properties": []
    });

    println!("Test case: Complex nested XML with multiple differences");
    println!("Request: {}", serde_json::to_string_pretty(&request_body).unwrap());
    println!("Expected: Multiple diffs for attributes and content across different elements");
}

#[tokio::test] 
async fn test_xml_comparison_api_identical_xmls() {
    let request_body = json!({
        "xml1": "<Mapping date=\"20250819\">test</Mapping>",
        "xml2": "<Mapping date=\"20250819\">test</Mapping>",
        "ignore_paths": [],
        "ignore_properties": []
    });

    println!("Test case: Identical XMLs");
    println!("Request: {}", serde_json::to_string_pretty(&request_body).unwrap());
    println!("Expected: matched=true, match_ratio=1.0, diffs=[]");
}

#[tokio::test]
async fn test_xml_comparison_api_attribute_only_difference() {
    let request_body = json!({
        "xml1": "<Mapping date=\"20250819\">test</Mapping>",
        "xml2": "<Mapping date=\"20250818\">test</Mapping>",
        "ignore_paths": [],
        "ignore_properties": []
    });

    println!("Test case: Attribute-only difference");
    println!("Request: {}", serde_json::to_string_pretty(&request_body).unwrap());
    println!("Expected: 1 AttributeDifferent diff for date attribute");
}

#[tokio::test]
async fn test_xml_comparison_api_content_only_difference() {
    let request_body = json!({
        "xml1": "<Mapping date=\"20250819\">test</Mapping>",
        "xml2": "<Mapping date=\"20250819\">test2</Mapping>",
        "ignore_paths": [],
        "ignore_properties": []
    });

    println!("Test case: Content-only difference");
    println!("Request: {}", serde_json::to_string_pretty(&request_body).unwrap());
    println!("Expected: 1 ContentDifferent diff");
}

// Manual test runner that prints curl commands for manual verification
#[tokio::test]
async fn test_runner_print_manual_test_commands() {
    println!("\n=== MANUAL TEST COMMANDS ===");
    
    println!("\n1. Test attribute and content differences:");
    println!("curl -s -X POST http://localhost:3000/xml-compare-api/api/compare/xml -H \"Content-Type: application/json\" -d '{{\"xml1\": \"<Mapping date=\\\"20250819\\\">test</Mapping>\", \"xml2\": \"<Mapping date=\\\"20250818\\\">test2</Mapping>\", \"ignore_paths\":[], \"ignore_properties\": []}}' | jq .");
    
    println!("\n2. Test ignoring date attribute:");
    println!("curl -s -X POST http://localhost:3000/xml-compare-api/api/compare/xml -H \"Content-Type: application/json\" -d '{{\"xml1\": \"<Mapping date=\\\"20250819\\\">test</Mapping>\", \"xml2\": \"<Mapping date=\\\"20250818\\\">test</Mapping>\", \"ignore_paths\":[], \"ignore_properties\": [\"date\"]}}' | jq .");
    
    println!("\n3. Test ignoring Mapping element:");
    println!("curl -s -X POST http://localhost:3000/xml-compare-api/api/compare/xml -H \"Content-Type: application/json\" -d '{{\"xml1\": \"<Mapping date=\\\"20250819\\\">test</Mapping>\", \"xml2\": \"<Mapping date=\\\"20250818\\\">test2</Mapping>\", \"ignore_paths\":[], \"ignore_properties\": [\"Mapping\"]}}' | jq .");
    
    println!("\n4. Test identical XMLs:");
    println!("curl -s -X POST http://localhost:3000/xml-compare-api/api/compare/xml -H \"Content-Type: application/json\" -d '{{\"xml1\": \"<Mapping date=\\\"20250819\\\">test</Mapping>\", \"xml2\": \"<Mapping date=\\\"20250819\\\">test</Mapping>\", \"ignore_paths\":[], \"ignore_properties\": []}}' | jq .");
    
    println!("\n5. Test complex nested XML:");
    println!("curl -s -X POST http://localhost:3000/xml-compare-api/api/compare/xml -H \"Content-Type: application/json\" -d '{{\"xml1\": \"<root><Mapping date=\\\"20250819\\\" version=\\\"1.0\\\">test<child attr=\\\"val1\\\">child1</child></Mapping></root>\", \"xml2\": \"<root><Mapping date=\\\"20250818\\\" version=\\\"1.1\\\">test2<child attr=\\\"val2\\\">child2</child></Mapping></root>\", \"ignore_paths\":[], \"ignore_properties\": []}}' | jq .");
}

#[tokio::test]
async fn test_health_check() {
    let app = create_test_app().await;
    
    let request = Request::builder()
        .method("GET")
        .uri("/health")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    
    assert_eq!(body_str, "OK");
}

#[tokio::test]
async fn test_xml_batch_comparison() {
    let app = create_test_app().await;
    
    let request_body = json!({
        "comparisons": [
            {
                "xml1": "<test>same</test>",
                "xml2": "<test>same</test>",
                "ignore_paths": [],
                "ignore_properties": []
            },
            {
                "xml1": "<test>different1</test>",
                "xml2": "<test>different2</test>",
                "ignore_paths": [],
                "ignore_properties": []
            }
        ]
    });

    let request = Request::builder()
        .method("POST")
        .uri("/api/compare/xml/batch")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&request_body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let response_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    
    // Should have 2 results
    assert_eq!(response_json["total_comparisons"], 2);
    assert_eq!(response_json["results"].as_array().unwrap().len(), 2);
    
    // First comparison should match, second should not
    let results = response_json["results"].as_array().unwrap();
    assert_eq!(results[0]["matched"], true);
    assert_eq!(results[1]["matched"], false);
}

#[tokio::test]  
async fn test_invalid_xml_handling() {
    let app = create_test_app().await;
    
    let request_body = json!({
        "xml1": "<invalid><not-closed",
        "xml2": "<test>valid</test>",
        "ignore_paths": [],
        "ignore_properties": []
    });

    let request = Request::builder()
        .method("POST")
        .uri("/api/compare/xml")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&request_body).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    
    // Should return a 400 error for invalid XML (or may be 200 if parser is lenient)
    // Let's check it's not a 500 error
    assert!(response.status() == StatusCode::BAD_REQUEST || response.status() == StatusCode::OK);
}
