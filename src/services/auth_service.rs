use crate::models::{AppError, AppResult, Session, SessionStore, LoginRequest, LoginResponse};
use crate::services::HttpClientService;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

pub struct AuthService {
    session_store: SessionStore,
    http_client: Arc<HttpClientService>,
}

impl AuthService {
    pub fn new(http_client: Arc<HttpClientService>) -> Self {
        Self {
            session_store: Arc::new(RwLock::new(HashMap::new())),
            http_client,
        }
    }

    pub async fn login(&self, request: &LoginRequest) -> AppResult<LoginResponse> {
        // Validate URL
        if !self.is_valid_url(&request.url) {
            return Err(AppError::InvalidUrl(request.url.clone()));
        }

        // Attempt authentication
        let session = self.http_client
            .authenticate(&request.url, &request.username, &request.password)
            .await?;

        // Store session
        {
            let mut sessions = self.session_store.write().await;
            sessions.insert(session.id.clone(), session.clone());
        }

        Ok(LoginResponse {
            session_id: session.id,
            cookies: session.cookies,
            expires_at: session.expires_at.to_rfc3339(),
        })
    }

    pub async fn get_session(&self, session_id: &str) -> AppResult<Option<Session>> {
        let sessions = self.session_store.read().await;
        Ok(sessions.get(session_id).cloned())
    }



    pub async fn logout(&self, session_id: &str) -> AppResult<()> {
        let mut sessions = self.session_store.write().await;
        sessions.remove(session_id);
        Ok(())
    }

    pub async fn cleanup_expired_sessions(&self) {
        let mut sessions = self.session_store.write().await;
        sessions.retain(|_, session| !session.is_expired());
    }

    fn is_valid_url(&self, url: &str) -> bool {
        url.starts_with("http://") || url.starts_with("https://")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::{method, path, header};

    #[tokio::test]
    async fn test_login_success() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("POST"))
            .and(path("/login"))
            .and(header("Authorization", "Basic dGVzdDpwYXNzd29yZA==")) // test:password
            .respond_with(ResponseTemplate::new(200)
                .insert_header("set-cookie", "session=abc123; HttpOnly"))
            .mount(&mock_server)
            .await;

        let http_client = Arc::new(HttpClientService::new());
        let auth_service = AuthService::new(http_client);
        
        let request = LoginRequest {
            url: format!("{}/login", mock_server.uri()),
            username: "test".to_string(),
            password: "password".to_string(),
        };

        let result = auth_service.login(&request).await;
        assert!(result.is_ok());
        
        let response = result.unwrap();
        assert!(!response.session_id.is_empty());
        assert!(!response.cookies.is_empty());
    }

    #[tokio::test]
    async fn test_login_invalid_url() {
        let http_client = Arc::new(HttpClientService::new());
        let auth_service = AuthService::new(http_client);
        
        let request = LoginRequest {
            url: "invalid-url".to_string(),
            username: "test".to_string(),
            password: "password".to_string(),
        };

        let result = auth_service.login(&request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_session_retrieval() {
        let http_client = Arc::new(HttpClientService::new());
        let auth_service = AuthService::new(http_client);
        
        // Test with non-existent session
        let result = auth_service.get_session("non-existent").await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }
}