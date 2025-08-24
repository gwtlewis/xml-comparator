use crate::models::{AppError, AppResult, Session};
use reqwest::Client;
use base64::{Engine as _, engine::general_purpose};

pub struct HttpClientService {
    client: Client,
}

impl HttpClientService {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    pub async fn download_xml(
        &self, 
        url: &str, 
        auth_service: Option<&crate::services::AuthService>,
        session_id: Option<&str>
    ) -> AppResult<String> {
        let mut request = self.client.get(url);

        // Add cookies if session exists
        if let (Some(auth_service), Some(session_id)) = (auth_service, session_id) {
            if let Some(session) = auth_service.get_session(session_id).await? {
                for cookie in &session.cookies {
                    request = request.header("Cookie", cookie);
                }
            }
        }

        let response = request.send().await.map_err(|e| AppError::HttpError(e.to_string()))?;
        
        if !response.status().is_success() {
            return Err(AppError::InternalError(
                format!("HTTP request failed with status: {}", response.status())
            ));
        }

        let content = response.text().await.map_err(|e| AppError::HttpError(e.to_string()))?;
        Ok(content)
    }

    // Note: batch download method removed as it's not used and would need significant refactoring
    // to work with the new auth service pattern

    pub async fn authenticate(
        &self,
        url: &str,
        username: &str,
        password: &str,
    ) -> AppResult<Session> {
        // Create base64 encoded credentials for basic auth
        let credentials = format!("{}:{}", username, password);
        let encoded_credentials = general_purpose::STANDARD.encode(credentials.as_bytes());
        let auth_header = format!("Basic {}", encoded_credentials);

        // Try POST first
        let post_result = self.try_authenticate_with_method(url, &auth_header, "POST").await;
        
        match post_result {
            Ok(session) => Ok(session),
            Err(post_error) => {
                // If POST fails, try GET
                tracing::info!("POST authentication failed for {}: {}, trying GET", url, post_error);
                self.try_authenticate_with_method(url, &auth_header, "GET").await
            }
        }
    }

    async fn try_authenticate_with_method(
        &self,
        url: &str,
        auth_header: &str,
        method: &str,
    ) -> AppResult<Session> {
        let request_builder = match method {
            "POST" => self.client.post(url),
            "GET" => self.client.get(url),
            _ => return Err(AppError::InternalError(format!("Unsupported HTTP method: {}", method))),
        };

        let response = request_builder
            .header("Authorization", auth_header)
            .send()
            .await
            .map_err(|e| {
                AppError::HttpError(format!(
                    "{} request failed: {} (URL: {})", 
                    method, e.to_string(), url
                ))
            })?;

        let status = response.status();
        
        if !status.is_success() {
            // Extract detailed error information
            let error_body = response.text().await.unwrap_or_else(|_| "Unable to read error response".to_string());
            
            let error_message = match status.as_u16() {
                401 => format!("Authentication failed ({}): Invalid credentials for {}", method, url),
                403 => format!("Access forbidden ({}): Insufficient permissions for {}", method, url),
                404 => format!("Endpoint not found ({}): Authentication endpoint does not exist at {}", method, url),
                500..=599 => format!("Server error ({}): Remote server error (status: {}) for {}", method, status, url),
                _ => format!("Authentication failed ({}): HTTP {} - {} for {}", method, status, error_body, url),
            };

            return Err(AppError::AuthError(error_message));
        }

        // Extract cookies from successful response
        let cookies: Vec<String> = response
            .headers()
            .get_all("set-cookie")
            .iter()
            .filter_map(|header| header.to_str().ok().map(|s| s.to_string()))
            .collect();

        let session = Session::new(url.to_string(), cookies);
        Ok(session)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::{method, path, header};

    #[tokio::test]
    async fn test_download_xml_success() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("GET"))
            .and(path("/test.xml"))
            .respond_with(ResponseTemplate::new(200).set_body_string("<test>content</test>"))
            .mount(&mock_server)
            .await;

        let service = HttpClientService::new();
        let url = format!("{}/test.xml", mock_server.uri());
        
        let result = service.download_xml(&url, None, None).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "<test>content</test>");
    }

    #[tokio::test]
    async fn test_download_xml_not_found() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("GET"))
            .and(path("/notfound.xml"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        let service = HttpClientService::new();
        let url = format!("{}/notfound.xml", mock_server.uri());
        
        let result = service.download_xml(&url, None, None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_authenticate_success_with_post() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("POST"))
            .and(path("/auth"))
            .and(header("Authorization", "Basic dGVzdDpwYXNzd29yZA==")) // test:password
            .respond_with(ResponseTemplate::new(200)
                .insert_header("set-cookie", "session=abc123; HttpOnly"))
            .mount(&mock_server)
            .await;

        let service = HttpClientService::new();
        let url = format!("{}/auth", mock_server.uri());
        
        let result = service.authenticate(&url, "test", "password").await;
        assert!(result.is_ok());
        
        let session = result.unwrap();
        assert_eq!(session.url, url);
        assert!(!session.cookies.is_empty());
        assert_eq!(session.cookies[0], "session=abc123; HttpOnly");
    }

    #[tokio::test]
    async fn test_authenticate_success_with_get_fallback() {
        let mock_server = MockServer::start().await;
        
        // Mock POST to fail with 405 Method Not Allowed
        Mock::given(method("POST"))
            .and(path("/auth"))
            .respond_with(ResponseTemplate::new(405))
            .mount(&mock_server)
            .await;

        // Mock GET to succeed
        Mock::given(method("GET"))
            .and(path("/auth"))
            .and(header("Authorization", "Basic dGVzdDpwYXNzd29yZA==")) // test:password
            .respond_with(ResponseTemplate::new(200)
                .insert_header("set-cookie", "session=xyz789; HttpOnly"))
            .mount(&mock_server)
            .await;

        let service = HttpClientService::new();
        let url = format!("{}/auth", mock_server.uri());
        
        let result = service.authenticate(&url, "test", "password").await;
        assert!(result.is_ok());
        
        let session = result.unwrap();
        assert_eq!(session.url, url);
        assert!(!session.cookies.is_empty());
        assert_eq!(session.cookies[0], "session=xyz789; HttpOnly");
    }

    #[tokio::test]
    async fn test_authenticate_invalid_credentials() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("POST"))
            .and(path("/auth"))
            .and(header("Authorization", "Basic dGVzdDpwYXNzd29yZA==")) // test:password
            .respond_with(ResponseTemplate::new(401)
                .set_body_string("Invalid credentials"))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/auth"))
            .and(header("Authorization", "Basic dGVzdDpwYXNzd29yZA==")) // test:password
            .respond_with(ResponseTemplate::new(401)
                .set_body_string("Invalid credentials"))
            .mount(&mock_server)
            .await;

        let service = HttpClientService::new();
        let url = format!("{}/auth", mock_server.uri());
        
        let result = service.authenticate(&url, "test", "password").await;
        assert!(result.is_err());
        
        if let AppError::AuthError(error_msg) = result.unwrap_err() {
            assert!(error_msg.contains("Authentication failed (GET): Invalid credentials"));
        } else {
            panic!("Expected AuthError");
        }
    }

    #[tokio::test]
    async fn test_authenticate_forbidden() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("POST"))
            .and(path("/auth"))
            .and(header("Authorization", "Basic dGVzdDpwYXNzd29yZA==")) // test:password
            .respond_with(ResponseTemplate::new(403)
                .set_body_string("Insufficient permissions"))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/auth"))
            .and(header("Authorization", "Basic dGVzdDpwYXNzd29yZA==")) // test:password
            .respond_with(ResponseTemplate::new(403)
                .set_body_string("Insufficient permissions"))
            .mount(&mock_server)
            .await;

        let service = HttpClientService::new();
        let url = format!("{}/auth", mock_server.uri());
        
        let result = service.authenticate(&url, "test", "password").await;
        assert!(result.is_err());
        
        if let AppError::AuthError(error_msg) = result.unwrap_err() {
            assert!(error_msg.contains("Access forbidden (GET): Insufficient permissions"));
        } else {
            panic!("Expected AuthError");
        }
    }

    #[tokio::test]
    async fn test_authenticate_endpoint_not_found() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("POST"))
            .and(path("/auth"))
            .and(header("Authorization", "Basic dGVzdDpwYXNzd29yZA==")) // test:password
            .respond_with(ResponseTemplate::new(404)
                .set_body_string("Endpoint not found"))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/auth"))
            .and(header("Authorization", "Basic dGVzdDpwYXNzd29yZA==")) // test:password
            .respond_with(ResponseTemplate::new(404)
                .set_body_string("Endpoint not found"))
            .mount(&mock_server)
            .await;

        let service = HttpClientService::new();
        let url = format!("{}/auth", mock_server.uri());
        
        let result = service.authenticate(&url, "test", "password").await;
        assert!(result.is_err());
        
        if let AppError::AuthError(error_msg) = result.unwrap_err() {
            assert!(error_msg.contains("Endpoint not found (GET): Authentication endpoint does not exist"));
        } else {
            panic!("Expected AuthError");
        }
    }

    #[tokio::test]
    async fn test_authenticate_server_error() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("POST"))
            .and(path("/auth"))
            .and(header("Authorization", "Basic dGVzdDpwYXNzd29yZA==")) // test:password
            .respond_with(ResponseTemplate::new(500)
                .set_body_string("Internal server error"))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/auth"))
            .and(header("Authorization", "Basic dGVzdDpwYXNzd29yZA==")) // test:password
            .respond_with(ResponseTemplate::new(500)
                .set_body_string("Internal server error"))
            .mount(&mock_server)
            .await;

        let service = HttpClientService::new();
        let url = format!("{}/auth", mock_server.uri());
        
        let result = service.authenticate(&url, "test", "password").await;
        assert!(result.is_err());
        
        if let AppError::AuthError(error_msg) = result.unwrap_err() {
            assert!(error_msg.contains("Server error (GET): Remote server error (status: 500"));
        } else {
            panic!("Expected AuthError");
        }
    }
}