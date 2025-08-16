use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct XmlComparisonRequest {
    pub xml1: String,
    pub xml2: String,
    pub ignore_paths: Option<Vec<String>>,
    pub ignore_properties: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct XmlComparisonResponse {
    pub matched: bool,
    pub match_ratio: f64,
    pub diffs: Vec<XmlDiff>,
    pub total_elements: usize,
    pub matched_elements: usize,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct XmlDiff {
    pub path: String,
    pub diff_type: DiffType,
    pub expected: Option<String>,
    pub actual: Option<String>,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub enum DiffType {
    ElementMissing,
    ElementExtra,
    AttributeDifferent,
    ContentDifferent,
    StructureDifferent,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UrlComparisonRequest {
    pub url1: String,
    pub url2: String,
    pub ignore_paths: Option<Vec<String>>,
    pub ignore_properties: Option<Vec<String>>,
    pub auth_credentials: Option<AuthCredentials>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AuthCredentials {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BatchXmlComparisonRequest {
    pub comparisons: Vec<XmlComparisonRequest>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BatchUrlComparisonRequest {
    pub comparisons: Vec<UrlComparisonRequest>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BatchComparisonResponse {
    pub results: Vec<XmlComparisonResponse>,
    pub total_comparisons: usize,
    pub successful_comparisons: usize,
    pub failed_comparisons: usize,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct LoginRequest {
    pub url: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct LoginResponse {
    pub session_id: String,
    pub cookies: Vec<String>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}