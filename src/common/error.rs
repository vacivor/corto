use std::collections::HashMap;

use axum::{http::StatusCode, response::{IntoResponse, Response}};

use crate::problem::ProblemDetail;

#[derive(Debug)]
pub struct ValidationErrors {
    pub errors: Vec<ValidationError>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationError {
    pub field: String,
    pub code: String,
    pub message: String,
}

impl ValidationErrors {
    pub fn single(field: impl Into<String>, code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            errors: vec![ValidationError {
                field: field.into(),
                code: code.into(),
                message: message.into(),
            }],
        }
    }
}

#[derive(Debug)]
pub enum AppError {
    InvalidInput { detail: String, errors: Option<ValidationErrors> },
    NotFound { detail: String },
    Conflict { detail: String },
    Gone { detail: String },
    Internal { detail: String },
}

impl AppError {
    pub fn bad_request(detail: impl Into<String>) -> Self {
        Self::InvalidInput {
            detail: detail.into(),
            errors: None,
        }
    }

    pub fn bad_request_with_errors(detail: impl Into<String>, errors: ValidationErrors) -> Self {
        Self::InvalidInput {
            detail: detail.into(),
            errors: Some(errors),
        }
    }

    pub fn not_found(detail: impl Into<String>) -> Self {
        Self::NotFound { detail: detail.into() }
    }

    pub fn conflict(detail: impl Into<String>) -> Self {
        Self::Conflict { detail: detail.into() }
    }

    pub fn gone(detail: impl Into<String>) -> Self {
        Self::Gone { detail: detail.into() }
    }

    pub fn internal(detail: impl Into<String>) -> Self {
        Self::Internal { detail: detail.into() }
    }

    fn status(&self) -> StatusCode {
        match self {
            Self::InvalidInput { .. } => StatusCode::BAD_REQUEST,
            Self::NotFound { .. } => StatusCode::NOT_FOUND,
            Self::Conflict { .. } => StatusCode::CONFLICT,
            Self::Gone { .. } => StatusCode::GONE,
            Self::Internal { .. } => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn title(&self) -> &'static str {
        match self {
            Self::InvalidInput { .. } => "Invalid input",
            Self::NotFound { .. } => "Not found",
            Self::Conflict { .. } => "Conflict",
            Self::Gone { .. } => "Gone",
            Self::Internal { .. } => "Internal error",
        }
    }

    fn detail(&self) -> &str {
        match self {
            Self::InvalidInput { detail, .. }
            | Self::NotFound { detail }
            | Self::Conflict { detail }
            | Self::Gone { detail }
            | Self::Internal { detail } => detail,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status();
        let mut problem = ProblemDetail {
            r#type: "about:blank".to_string(),
            title: self.title().to_string(),
            status: status.as_u16(),
            detail: self.detail().to_string(),
            instance: None,
            extensions: HashMap::new(),
        };

        if let Self::InvalidInput { errors: Some(errors), .. } = self {
            problem = problem.with_errors(errors.errors);
        }

        let body = serde_json::to_string(&problem).unwrap_or_else(|_| {
            let fallback = ProblemDetail {
                r#type: "about:blank".to_string(),
                title: "Internal error".to_string(),
                status: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
                detail: "Failed to serialize problem details".to_string(),
                instance: None,
                extensions: HashMap::new(),
            };
            serde_json::to_string(&fallback).unwrap()
        });

        (
            status,
            [(axum::http::header::CONTENT_TYPE, "application/problem+json")],
            body,
        )
            .into_response()
    }
}
