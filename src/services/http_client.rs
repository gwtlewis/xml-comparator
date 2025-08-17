use crate::models::{AppError, AppResult, Session};
use reqwest::Client;

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
        let response = self
            .client
            .post(url)
            .form(&[("username", username), ("password", password)])
            .send()
            .await.map_err(|e| AppError::HttpError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(AppError::AuthError("Authentication failed".to_string()));
        }

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
    use wiremock::matchers::{method, path};

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
}