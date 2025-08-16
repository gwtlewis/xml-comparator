use axum::{
    routing::post,
    Router,
    Json,
    http::Method,
};
use serde::{Deserialize, Serialize};
use tower_http::cors::{CorsLayer, Any};
use std::collections::HashMap;
use quick_xml::Reader;
use quick_xml::events::Event;

#[derive(Debug, Serialize, Deserialize)]
pub struct XmlComparisonRequest {
    pub xml1: String,
    pub xml2: String,
    pub ignore_paths: Option<Vec<String>>,
    pub ignore_properties: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct XmlComparisonResponse {
    pub matched: bool,
    pub match_ratio: f64,
    pub diffs: Vec<XmlDiff>,
    pub total_elements: usize,
    pub matched_elements: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct XmlDiff {
    pub path: String,
    pub diff_type: String,
    pub expected: Option<String>,
    pub actual: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct XmlElement {
    pub name: String,
    pub attributes: HashMap<String, String>,
    pub content: Option<String>,
    pub path: String,
}

pub struct XmlComparisonService;

impl XmlComparisonService {
    pub fn new() -> Self {
        Self
    }

    pub fn compare_xmls(&self, request: &XmlComparisonRequest) -> Result<XmlComparisonResponse, String> {
        let xml1_elements = self.parse_xml(&request.xml1)?;
        let xml2_elements = self.parse_xml(&request.xml2)?;

        let mut diffs = Vec::new();
        let mut matched_elements = 0;
        let total_elements = xml1_elements.len().max(xml2_elements.len());

        // Compare elements
        for (path, element1) in &xml1_elements {
            if let Some(element2) = xml2_elements.get(path) {
                if self.elements_match(element1, element2, &request.ignore_paths, &request.ignore_properties) {
                    matched_elements += 1;
                } else {
                    diffs.push(self.create_diff(path, element1, element2, &request.ignore_paths, &request.ignore_properties));
                }
            } else {
                diffs.push(XmlDiff {
                    path: path.clone(),
                    diff_type: "ElementMissing".to_string(),
                    expected: Some(format!("{:?}", element1)),
                    actual: None,
                    message: "Element missing in second XML".to_string(),
                });
            }
        }

        // Check for extra elements in xml2
        for (path, element2) in &xml2_elements {
            if !xml1_elements.contains_key(path) {
                diffs.push(XmlDiff {
                    path: path.clone(),
                    diff_type: "ElementExtra".to_string(),
                    expected: None,
                    actual: Some(format!("{:?}", element2)),
                    message: "Extra element in second XML".to_string(),
                });
            }
        }

        let match_ratio = if total_elements > 0 {
            matched_elements as f64 / total_elements as f64
        } else {
            1.0
        };

        Ok(XmlComparisonResponse {
            matched: diffs.is_empty(),
            match_ratio,
            diffs,
            total_elements,
            matched_elements,
        })
    }

    fn parse_xml(&self, xml_content: &str) -> Result<HashMap<String, XmlElement>, String> {
        let mut reader = Reader::from_str(xml_content);
        reader.trim_text(true);

        let mut elements = HashMap::new();
        let mut buf = Vec::new();
        let mut current_path = String::new();
        let mut stack = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => {
                    let name = String::from_utf8_lossy(e.name().into_inner()).to_string();
                    let path = if current_path.is_empty() {
                        format!("/{}", name)
                    } else {
                        format!("{}/{}", current_path, name)
                    };

                    let mut attributes = HashMap::new();
                    for attr in e.attributes() {
                        if let Ok(attr) = attr {
                            let key = String::from_utf8_lossy(attr.key.into_inner()).to_string();
                            let value = String::from_utf8_lossy(&attr.value).to_string();
                            attributes.insert(key, value);
                        }
                    }

                    let element = XmlElement {
                        name: name.clone(),
                        attributes,
                        content: None,
                        path: path.clone(),
                    };

                    elements.insert(path.clone(), element);
                    stack.push(path.clone());
                    current_path = path;
                }
                Ok(Event::Text(e)) => {
                    if let Some(path) = stack.last() {
                        if let Some(element) = elements.get_mut(path) {
                            element.content = Some(String::from_utf8_lossy(&e).trim().to_string());
                        }
                    }
                }
                Ok(Event::End(_)) => {
                    if let Some(path) = stack.pop() {
                        current_path = stack.last().cloned().unwrap_or_default();
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(e.to_string()),
                _ => {}
            }
        }

        Ok(elements)
    }

    fn elements_match(
        &self,
        element1: &XmlElement,
        element2: &XmlElement,
        ignore_paths: &Option<Vec<String>>,
        ignore_properties: &Option<Vec<String>>,
    ) -> bool {
        // Check if this path should be ignored
        if let Some(ignore_paths) = ignore_paths {
            if ignore_paths.iter().any(|path| element1.path.contains(path)) {
                return true;
            }
        }

        // Check if this element name should be ignored
        if let Some(ignore_properties) = ignore_properties {
            if ignore_properties.iter().any(|prop| &element1.name == prop) {
                return true;
            }
        }

        // Compare names
        if element1.name != element2.name {
            return false;
        }

        // Compare content (if not ignored)
        if let Some(ignore_properties) = ignore_properties {
            if !ignore_properties.iter().any(|prop| &element1.name == prop) {
                if element1.content != element2.content {
                    return false;
                }
            }
        } else {
            if element1.content != element2.content {
                return false;
            }
        }

        // Compare attributes (if not ignored)
        for (key, value1) in &element1.attributes {
            if let Some(ignore_properties) = ignore_properties {
                if ignore_properties.iter().any(|prop| key == prop) {
                    continue;
                }
            }
            if let Some(value2) = element2.attributes.get(key) {
                if value1 != value2 {
                    return false;
                }
            } else {
                return false;
            }
        }

        true
    }

    fn create_diff(
        &self,
        path: &str,
        element1: &XmlElement,
        element2: &XmlElement,
        ignore_paths: &Option<Vec<String>>,
        ignore_properties: &Option<Vec<String>>,
    ) -> XmlDiff {
        // Check content differences
        if let Some(ignore_properties) = ignore_properties {
            if !ignore_properties.iter().any(|prop| &element1.name == prop) {
                if element1.content != element2.content {
                    return XmlDiff {
                        path: path.to_string(),
                        diff_type: "ContentDifferent".to_string(),
                        expected: element1.content.clone(),
                        actual: element2.content.clone(),
                        message: "Content differs".to_string(),
                    };
                }
            }
        } else {
            if element1.content != element2.content {
                return XmlDiff {
                    path: path.to_string(),
                    diff_type: "ContentDifferent".to_string(),
                    expected: element1.content.clone(),
                    actual: element2.content.clone(),
                    message: "Content differs".to_string(),
                };
            }
        }

        // Check attribute differences
        for (key, value1) in &element1.attributes {
            if let Some(ignore_properties) = ignore_properties {
                if ignore_properties.iter().any(|prop| key == prop) {
                    continue;
                }
            }
            if let Some(value2) = element2.attributes.get(key) {
                if value1 != value2 {
                    return XmlDiff {
                        path: path.to_string(),
                        diff_type: "AttributeDifferent".to_string(),
                        expected: Some(format!("{}={}", key, value1)),
                        actual: Some(format!("{}={}", key, value2)),
                        message: format!("Attribute '{}' differs", key),
                    };
                }
            }
        }

        XmlDiff {
            path: path.to_string(),
            diff_type: "StructureDifferent".to_string(),
            expected: Some(format!("{:?}", element1)),
            actual: Some(format!("{:?}", element2)),
            message: "Structure differs".to_string(),
        }
    }
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Configure CORS
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_origin(Any);

    // Create router
    let app = Router::new()
        .route("/api/compare/xml", post(compare_xmls))
        .layer(cors);

    // Start server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    tracing::info!("Server running on http://0.0.0.0:3000");

    axum::serve(listener, app).await.unwrap();
}

async fn compare_xmls(Json(request): Json<XmlComparisonRequest>) -> Json<XmlComparisonResponse> {
    let service = XmlComparisonService::new();
    match service.compare_xmls(&request) {
        Ok(response) => Json(response),
        Err(_) => Json(XmlComparisonResponse {
            matched: false,
            match_ratio: 0.0,
            diffs: vec![],
            total_elements: 0,
            matched_elements: 0,
        })
    }
}
