use crate::models::{AppError, AppResult, Session, SessionStore};
use reqwest::Client;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct HttpClientService {
    client: Client,
    session_store: SessionStore,
}

impl HttpClientService {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            session_store: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn download_xml(&self, url: &str, session_id: Option<&str>) -> AppResult<String> {
        let mut request = self.client.get(url);

        // Add cookies if session exists
        if let Some(session_id) = session_id {
            if let Some(session) = self.get_session(session_id).await? {
                for cookie in &session.cookies {
                    request = request.header("Cookie", cookie);
                }
            }
        }

        let response = request.send().await?;
        
        if !response.status().is_success() {
            return Err(AppError::HttpError(
                reqwest::Error::status(response.status())
            ));
        }

        let content = response.text().await?;
        Ok(content)
    }

    pub async fn download_xmls_batch(
        &self,
        urls: &[String],
        session_id: Option<&str>,
    ) -> AppResult<Vec<AppResult<String>>> {
        let mut futures = Vec::new();

        for url in urls {
            let url = url.clone();
            let session_id = session_id.map(|s| s.to_string());
            let client = self.client.clone();
            let session_store = self.session_store.clone();

            let future = tokio::spawn(async move {
                let mut request = client.get(&url);

                if let Some(session_id) = session_id {
                    if let Ok(sessions) = session_store.read().await {
                        if let Some(session) = sessions.get(&session_id) {
                            for cookie in &session.cookies {
                                request = request.header("Cookie", cookie);
                            }
                        }
                    }
                }

                match request.send().await {
                    Ok(response) => {
                        if response.status().is_success() {
                            match response.text().await {
                                Ok(content) => Ok(content),
                                Err(e) => Err(AppError::HttpError(e)),
                            }
                        } else {
                            Err(AppError::HttpError(
                                reqwest::Error::status(response.status())
                            ))
                        }
                    }
                    Err(e) => Err(AppError::HttpError(e)),
                }
            });

            futures.push(future);
        }

        let mut results = Vec::new();
        for future in futures {
            match future.await {
                Ok(result) => results.push(result),
                Err(_) => results.push(Err(AppError::InternalError("Task failed".to_string()))),
            }
        }

        Ok(results)
    }

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
            .await?;

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
        
        // Store session
        {
            let mut sessions = self.session_store.write().await;
            sessions.insert(session.id.clone(), session.clone());
        }

        Ok(session)
    }

    async fn get_session(&self, session_id: &str) -> AppResult<Option<Session>> {
        let sessions = self.session_store.read().await;
        Ok(sessions.get(session_id).cloned())
    }

    pub async fn cleanup_expired_sessions(&self) {
        let mut sessions = self.session_store.write().await;
        sessions.retain(|_, session| !session.is_expired());
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
        
        let result = service.download_xml(&url, None).await;
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
        
        let result = service.download_xml(&url, None).await;
        assert!(result.is_err());
    }
}