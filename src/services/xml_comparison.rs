use crate::models::{
    XmlComparisonRequest, XmlComparisonResponse, XmlDiff, DiffType, AppError, AppResult,
};
use quick_xml::Reader;
use quick_xml::events::Event;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct XmlElement {
    pub name: String,
    pub attributes: HashMap<String, String>,
    pub content: Option<String>,
}

#[derive(Clone)]
pub struct XmlComparisonService;

impl XmlComparisonService {
    pub fn new() -> Self {
        Self
    }

    pub fn compare_xmls(&self, request: &XmlComparisonRequest) -> AppResult<XmlComparisonResponse> {
        let xml1_elements = self.parse_xml(&request.xml1)?;
        let xml2_elements = self.parse_xml(&request.xml2)?;

        let mut diffs = Vec::new();
        let mut matched_elements = 0;
        let total_elements = xml1_elements.len().max(xml2_elements.len());

        // Compare elements
        for (path, element1) in &xml1_elements {
            if let Some(element2) = xml2_elements.get(path) {
                let element_diffs = self.create_element_diffs(path, element1, element2, &request.ignore_paths, &request.ignore_properties);
                if element_diffs.is_empty() {
                    matched_elements += 1;
                } else {
                    diffs.extend(element_diffs);
                }
            } else {
                diffs.push(XmlDiff {
                    path: path.clone(),
                    diff_type: DiffType::ElementMissing,
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
                    diff_type: DiffType::ElementExtra,
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

    fn parse_xml(&self, xml_content: &str) -> AppResult<HashMap<String, XmlElement>> {
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
                    if let Some(_path) = stack.pop() {
                        current_path = stack.last().cloned().unwrap_or_default();
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(AppError::XmlParseError(e.to_string())),
                _ => {}
            }
        }

        Ok(elements)
    }



    fn create_element_diffs(
        &self,
        path: &str,
        element1: &XmlElement,
        element2: &XmlElement,
        ignore_paths: &Option<Vec<String>>,
        ignore_properties: &Option<Vec<String>>,
    ) -> Vec<XmlDiff> {
        let mut diffs = Vec::new();

        // Check if this path should be ignored
        if let Some(ignore_paths) = ignore_paths {
            if ignore_paths.iter().any(|ignore_path| self.path_matches(path, ignore_path)) {
                return diffs;
            }
        }

        // Check if this element name should be ignored
        if let Some(ignore_properties) = ignore_properties {
            if ignore_properties.iter().any(|prop| &element1.name == prop) {
                return diffs;
            }
        }

        // Check content differences
        let content_ignored = if let Some(ignore_properties) = ignore_properties {
            ignore_properties.iter().any(|prop| &element1.name == prop)
        } else {
            false
        };

        if !content_ignored && element1.content != element2.content {
            diffs.push(XmlDiff {
                path: path.to_string(),
                diff_type: DiffType::ContentDifferent,
                expected: element1.content.clone(),
                actual: element2.content.clone(),
                message: "Content differs".to_string(),
            });
        }

        // Check attribute differences
        for (key, value1) in &element1.attributes {
            let attr_ignored = if let Some(ignore_properties) = ignore_properties {
                ignore_properties.iter().any(|prop| key == prop)
            } else {
                false
            };

            if !attr_ignored {
                if let Some(value2) = element2.attributes.get(key) {
                    if value1 != value2 {
                        diffs.push(XmlDiff {
                            path: path.to_string(),
                            diff_type: DiffType::AttributeDifferent,
                            expected: Some(format!("{}={}", key, value1)),
                            actual: Some(format!("{}={}", key, value2)),
                            message: format!("Attribute '{}' differs", key),
                        });
                    }
                } else {
                    diffs.push(XmlDiff {
                        path: path.to_string(),
                        diff_type: DiffType::AttributeDifferent,
                        expected: Some(format!("{}={}", key, value1)),
                        actual: None,
                        message: format!("Attribute '{}' missing in second XML", key),
                    });
                }
            }
        }

        // Check for extra attributes in element2
        for (key, value2) in &element2.attributes {
            let attr_ignored = if let Some(ignore_properties) = ignore_properties {
                ignore_properties.iter().any(|prop| key == prop)
            } else {
                false
            };

            if !attr_ignored && !element1.attributes.contains_key(key) {
                diffs.push(XmlDiff {
                    path: path.to_string(),
                    diff_type: DiffType::AttributeDifferent,
                    expected: None,
                    actual: Some(format!("{}={}", key, value2)),
                    message: format!("Extra attribute '{}' in second XML", key),
                });
            }
        }

        diffs
    }

    fn path_matches(&self, actual_path: &str, ignore_pattern: &str) -> bool {
        // Support exact path matching and simple wildcard patterns
        if ignore_pattern == actual_path {
            return true; // Exact match
        }
        
        // Support wildcard patterns (simple * at end)
        if ignore_pattern.ends_with("*") {
            let prefix = &ignore_pattern[..ignore_pattern.len() - 1];
            return actual_path.starts_with(prefix);
        }
        
        // Support path prefix matching (if pattern ends with /)
        if ignore_pattern.ends_with("/") {
            return actual_path.starts_with(ignore_pattern) || 
                   format!("{}/", actual_path).starts_with(ignore_pattern);
        }
        
        // Default: exact match only
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identical_xmls() {
        let service = XmlComparisonService::new();
        let request = XmlComparisonRequest {
            xml1: "<a c=\"C\"><child>hey</child></a>".to_string(),
            xml2: "<a c=\"C\"><child>hey</child></a>".to_string(),
            ignore_paths: None,
            ignore_properties: None,
        };

        let result = service.compare_xmls(&request).unwrap();
        assert!(result.matched);
        assert_eq!(result.match_ratio, 1.0);
        assert!(result.diffs.is_empty());
    }

    #[test]
    fn test_ignore_property() {
        let service = XmlComparisonService::new();
        let request = XmlComparisonRequest {
            xml1: "<a c=\"C\"><child>hey</child></a>".to_string(),
            xml2: "<a c=\"D\"><child>hey</child></a>".to_string(),
            ignore_paths: None,
            ignore_properties: Some(vec!["c".to_string()]),
        };

        let result = service.compare_xmls(&request).unwrap();
        assert!(result.matched);
        assert_eq!(result.match_ratio, 1.0);
        assert!(result.diffs.is_empty());
    }

    #[test]
    fn test_ignore_tag() {
        let service = XmlComparisonService::new();
        let request = XmlComparisonRequest {
            xml1: "<a c=\"C\"><child>hey</child></a>".to_string(),
            xml2: "<a c=\"C\"><child>yo</child></a>".to_string(),
            ignore_paths: None,
            ignore_properties: Some(vec!["child".to_string()]),
        };

        let result = service.compare_xmls(&request).unwrap();
        assert!(result.matched);
        assert_eq!(result.match_ratio, 1.0);
        assert!(result.diffs.is_empty());
    }

    #[test]
    fn test_different_xmls() {
        let service = XmlComparisonService::new();
        let request = XmlComparisonRequest {
            xml1: "<a c=\"C\"><child>hey</child></a>".to_string(),
            xml2: "<a c=\"D\"><child>yo</child></a>".to_string(),
            ignore_paths: None,
            ignore_properties: None,
        };

        let result = service.compare_xmls(&request).unwrap();
        assert!(!result.matched);
        assert!(result.match_ratio < 1.0);
        assert!(!result.diffs.is_empty());
    }

    #[test]
    fn test_attribute_and_content_differences() {
        let service = XmlComparisonService::new();
        let request = XmlComparisonRequest {
            xml1: "<CVAMapping date=\"20250819\">test</CVAMapping>".to_string(),
            xml2: "<CVAMapping date=\"20250818\">test2</CVAMapping>".to_string(),
            ignore_paths: Some(vec![]),
            ignore_properties: Some(vec![]),
        };

        let result = service.compare_xmls(&request).unwrap();
        assert!(!result.matched);
        assert_eq!(result.diffs.len(), 2); // Should have both attribute and content diffs
        
        // Check we have both types of diffs
        let has_content_diff = result.diffs.iter().any(|d| matches!(d.diff_type, DiffType::ContentDifferent));
        let has_attr_diff = result.diffs.iter().any(|d| matches!(d.diff_type, DiffType::AttributeDifferent));
        
        assert!(has_content_diff, "Should have content difference");
        assert!(has_attr_diff, "Should have attribute difference");
    }

    #[test]
    fn test_attribute_only_difference() {
        let service = XmlComparisonService::new();
        let request = XmlComparisonRequest {
            xml1: "<CVAMapping date=\"20250819\">test</CVAMapping>".to_string(),
            xml2: "<CVAMapping date=\"20250818\">test</CVAMapping>".to_string(),
            ignore_paths: None,
            ignore_properties: None,
        };

        let result = service.compare_xmls(&request).unwrap();
        assert!(!result.matched);
        assert_eq!(result.diffs.len(), 1);
        assert!(matches!(result.diffs[0].diff_type, DiffType::AttributeDifferent));
        assert_eq!(result.diffs[0].path, "/CVAMapping");
        assert!(result.diffs[0].message.contains("date"));
    }

    #[test]
    fn test_ignore_attribute_property() {
        let service = XmlComparisonService::new();
        let request = XmlComparisonRequest {
            xml1: "<CVAMapping date=\"20250819\">test</CVAMapping>".to_string(),
            xml2: "<CVAMapping date=\"20250818\">test</CVAMapping>".to_string(),
            ignore_paths: None,
            ignore_properties: Some(vec!["date".to_string()]),
        };

        let result = service.compare_xmls(&request).unwrap();
        assert!(result.matched);
        assert_eq!(result.diffs.len(), 0);
    }

    #[test]
    fn test_content_only_difference() {
        let service = XmlComparisonService::new();
        let request = XmlComparisonRequest {
            xml1: "<CVAMapping date=\"20250819\">test</CVAMapping>".to_string(),
            xml2: "<CVAMapping date=\"20250819\">test2</CVAMapping>".to_string(),
            ignore_paths: None,
            ignore_properties: None,
        };

        let result = service.compare_xmls(&request).unwrap();
        assert!(!result.matched);
        assert_eq!(result.diffs.len(), 1);
        assert!(matches!(result.diffs[0].diff_type, DiffType::ContentDifferent));
        assert_eq!(result.diffs[0].path, "/CVAMapping");
    }

    #[test]
    fn test_path_matching_exact() {
        let service = XmlComparisonService::new();
        assert!(service.path_matches("/root/child", "/root/child"));
        assert!(!service.path_matches("/root/child", "/root/other"));
    }

    #[test]
    fn test_path_matching_wildcard() {
        let service = XmlComparisonService::new();
        assert!(service.path_matches("/root/child/grandchild", "/root/*"));
        assert!(service.path_matches("/root/child", "/root/*"));
        assert!(!service.path_matches("/other/child", "/root/*"));
    }

    #[test]
    fn test_path_matching_prefix() {
        let service = XmlComparisonService::new();
        assert!(service.path_matches("/root/child/grandchild", "/root/"));
        assert!(service.path_matches("/root", "/root/"));
        assert!(!service.path_matches("/other", "/root/"));
    }

    #[test]
    fn test_ignore_paths_exact_match() {
        let service = XmlComparisonService::new();
        let request = XmlComparisonRequest {
            xml1: "<root><child>test1</child><other>test2</other></root>".to_string(),
            xml2: "<root><child>different</child><other>test2</other></root>".to_string(),
            ignore_paths: Some(vec!["/root/child".to_string()]),
            ignore_properties: None,
        };

        let result = service.compare_xmls(&request).unwrap();
        assert!(result.matched);
        assert_eq!(result.diffs.len(), 0);
    }

    #[test]
    fn test_ignore_paths_wildcard() {
        let service = XmlComparisonService::new();
        let request = XmlComparisonRequest {
            xml1: "<root><child><deep>test1</deep></child><other>test2</other></root>".to_string(),
            xml2: "<root><child><deep>different</deep></child><other>test2</other></root>".to_string(),
            ignore_paths: Some(vec!["/root/child/*".to_string()]),
            ignore_properties: None,
        };

        let result = service.compare_xmls(&request).unwrap();
        assert!(result.matched);
        assert_eq!(result.diffs.len(), 0);
    }
}