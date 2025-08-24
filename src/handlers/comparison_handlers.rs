use axum::{
    extract::State,
    Json,
};
use crate::models::{
    XmlComparisonRequest, XmlComparisonResponse, UrlComparisonRequest,
    BatchXmlComparisonRequest, BatchUrlComparisonRequest, BatchComparisonResponse,
    AppError, AppResult,
};
use crate::services::{XmlComparisonService, HttpClientService};
use std::sync::Arc;


pub type AppState = Arc<AppStateInner>;

#[derive(Clone)]
pub struct AppStateInner {
    pub xml_service: XmlComparisonService,
    pub http_client: Arc<HttpClientService>,
    pub auth_service: Arc<crate::services::AuthService>,
}

/// Compare two XML contents
#[utoipa::path(
    post,
    path = "/xml-compare-api/api/compare/xml",
    request_body = XmlComparisonRequest,
    responses(
        (status = 200, description = "XML comparison completed", body = XmlComparisonResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error")
    ),
    tag = "XML Comparison"
)]
pub async fn compare_xmls(
    State(state): State<AppState>,
    Json(request): Json<XmlComparisonRequest>,
) -> AppResult<Json<XmlComparisonResponse>> {
    let result = state.xml_service.compare_xmls(&request)?;
    Ok(Json(result))
}

/// Compare XMLs from two URLs
#[utoipa::path(
    post,
    path = "/xml-compare-api/api/compare/url",
    request_body = UrlComparisonRequest,
    responses(
        (status = 200, description = "URL XML comparison completed", body = XmlComparisonResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Authentication required"),
        (status = 500, description = "Internal server error")
    ),
    tag = "URL Comparison"
)]
pub async fn compare_urls(
    State(state): State<AppState>,
    Json(request): Json<UrlComparisonRequest>,
) -> AppResult<Json<XmlComparisonResponse>> {
    // Handle authentication - either use session_id or create new session from auth_credentials
    let session_id_string = if let Some(session_id) = &request.session_id {
        Some(session_id.clone())
    } else if let Some(auth_creds) = &request.auth_credentials {
        // Create a temporary session for this request
        let login_request = crate::models::LoginRequest {
            url: request.url1.clone(), // Use first URL as login URL
            username: auth_creds.username.clone(),
            password: auth_creds.password.clone(),
        };
        let login_response = state.auth_service.login(&login_request).await?;
        Some(login_response.session_id)
    } else {
        None
    };
    
    let session_id = session_id_string.as_deref();

    // Download XMLs from URLs
    let xml1 = state.http_client
        .download_xml(&request.url1, Some(&*state.auth_service), session_id)
        .await?;
    
    let xml2 = state.http_client
        .download_xml(&request.url2, Some(&*state.auth_service), session_id)
        .await?;

    // Create comparison request
    let comparison_request = XmlComparisonRequest {
        xml1,
        xml2,
        ignore_paths: request.ignore_paths,
        ignore_properties: request.ignore_properties,
    };

    let result = state.xml_service.compare_xmls(&comparison_request)?;
    Ok(Json(result))
}

/// Compare multiple XML pairs in batch
#[utoipa::path(
    post,
    path = "/xml-compare-api/api/compare/xml/batch",
    request_body = BatchXmlComparisonRequest,
    responses(
        (status = 200, description = "Batch XML comparison completed", body = BatchComparisonResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Batch Comparison"
)]
pub async fn compare_xmls_batch(
    State(state): State<AppState>,
    Json(request): Json<BatchXmlComparisonRequest>,
) -> AppResult<Json<BatchComparisonResponse>> {
    let mut results = Vec::new();
    let mut successful = 0;
    let mut failed = 0;

    let _total_comparisons = request.comparisons.len();
    for comparison in &request.comparisons {
        match state.xml_service.compare_xmls(&comparison) {
            Ok(result) => {
                results.push(result);
                successful += 1;
            }
            Err(_) => {
                failed += 1;
                // Add a failed result placeholder
                results.push(XmlComparisonResponse {
                    matched: false,
                    match_ratio: 0.0,
                    diffs: vec![],
                    total_elements: 0,
                    matched_elements: 0,
                });
            }
        }
    }

    Ok(Json(BatchComparisonResponse {
        results,
        total_comparisons: request.comparisons.len(),
        successful_comparisons: successful,
        failed_comparisons: failed,
    }))
}

/// Compare XMLs from multiple URL pairs in batch
#[utoipa::path(
    post,
    path = "/xml-compare-api/api/compare/url/batch",
    request_body = BatchUrlComparisonRequest,
    responses(
        (status = 200, description = "Batch URL comparison completed", body = BatchComparisonResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Authentication required"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Batch URL Comparison"
)]
pub async fn compare_urls_batch(
    State(state): State<AppState>,
    Json(request): Json<BatchUrlComparisonRequest>,
) -> AppResult<Json<BatchComparisonResponse>> {
    let mut results = Vec::new();
    let mut successful = 0;
    let mut failed = 0;

    // Process comparisons concurrently
    let mut futures = Vec::new();
    
    let _total_comparisons = request.comparisons.len();
    for comparison in request.comparisons.clone() {
        let state = state.clone();
        let future = tokio::spawn(async move {
            // Handle authentication for this comparison
            let session_id_string = if let Some(session_id) = &comparison.session_id {
                Some(session_id.clone())
            } else if let Some(auth_creds) = &comparison.auth_credentials {
                // Create a temporary session for this request
                let login_request = crate::models::LoginRequest {
                    url: comparison.url1.clone(),
                    username: auth_creds.username.clone(),
                    password: auth_creds.password.clone(),
                };
                match state.auth_service.login(&login_request).await {
                    Ok(login_response) => Some(login_response.session_id),
                    Err(_) => None,
                }
            } else {
                None
            };
            
            let session_id = session_id_string.as_deref();

            // Download XMLs from URLs
            let xml1_result = state.http_client
                .download_xml(&comparison.url1, Some(&*state.auth_service), session_id)
                .await;
            
            let xml2_result = state.http_client
                .download_xml(&comparison.url2, Some(&*state.auth_service), session_id)
                .await;

            match (xml1_result, xml2_result) {
                (Ok(xml1), Ok(xml2)) => {
                            let comparison_request = XmlComparisonRequest {
            xml1,
            xml2,
            ignore_paths: comparison.ignore_paths.clone(),
            ignore_properties: comparison.ignore_properties.clone(),
        };

                    state.xml_service.compare_xmls(&comparison_request)
                }
                _ => Err(AppError::InternalError("Failed to download XML from URL".to_string())),
            }
        });
        
        futures.push(future);
    }

    // Collect results
    for future in futures {
        match future.await {
            Ok(Ok(result)) => {
                results.push(result);
                successful += 1;
            }
            _ => {
                failed += 1;
                results.push(XmlComparisonResponse {
                    matched: false,
                    match_ratio: 0.0,
                    diffs: vec![],
                    total_elements: 0,
                    matched_elements: 0,
                });
            }
        }
    }

    Ok(Json(BatchComparisonResponse {
        results,
        total_comparisons: request.comparisons.len(),
        successful_comparisons: successful,
        failed_comparisons: failed,
    }))
}