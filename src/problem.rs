use std::collections::HashMap;

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ProblemDetail {
    #[serde(rename = "type")]
    pub r#type: String,
    pub title: String,
    pub status: u16,
    pub detail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance: Option<String>,
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    pub extensions: HashMap<String, serde_json::Value>,
}

impl ProblemDetail {
    pub fn with_errors<T: serde::Serialize>(mut self, errors: T) -> Self {
        self.extensions
            .insert("errors".to_string(), serde_json::json!(errors));
        self
    }
}

impl ProblemDetail {
    pub fn add_extension(mut self, extension: HashMap<String, serde_json::Value>) -> Self {
        self.extensions.extend(extension);
        self
    }
}
