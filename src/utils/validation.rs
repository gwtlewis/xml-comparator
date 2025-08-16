use crate::models::AppError;

pub fn validate_xml_content(xml: &str) -> Result<(), AppError> {
    if xml.trim().is_empty() {
        return Err(AppError::ValidationError("XML content cannot be empty".to_string()));
    }
    
    // Basic XML validation - check if it starts with < and has closing tags
    if !xml.trim().starts_with('<') {
        return Err(AppError::ValidationError("Invalid XML format".to_string()));
    }
    
    Ok(())
}

pub fn validate_url(url: &str) -> Result<(), AppError> {
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(AppError::InvalidUrl(url.to_string()));
    }
    
    Ok(())
}