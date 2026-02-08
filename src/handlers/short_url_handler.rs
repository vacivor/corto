use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
    Json,
};
use serde::{Deserialize, Serialize};
use crate::{
    common::error::{AppError, ValidationErrors},
    common::validation::validate_url,
    app::AppState,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateShortUrlRequest {
    pub url: String,
    pub expires_at: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortUrlResponse {
    pub id: i64,
    pub short_code: String,
    pub url: String,
    pub status: i16,
    pub is_deleted: i16,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
    pub expires_at: Option<String>,
    pub visit_count: i64,
    pub short_url: Option<String>,
}

pub async fn create_short_url(
    State(state): State<AppState>,
    Json(payload): Json<CreateShortUrlRequest>,
) -> Result<impl IntoResponse, AppError> {
    validate_url(&payload.url, "url")?;
    let expires_at = parse_expires_at(payload.expires_at)?;

    let model = state
        .short_url_service
        .create_short_url(payload.url, expires_at)
        .await?;
    let code = model.short_code.clone().unwrap_or_else(|| "".to_string());

    let response = ShortUrlResponse {
        id: model.id,
        short_code: code.clone(),
        url: model.original_url,
        status: model.status,
        is_deleted: model.is_deleted,
        created_at: model.created_at.to_rfc3339(),
        updated_at: model.updated_at.to_rfc3339(),
        deleted_at: model.deleted_at.map(|t| t.to_rfc3339()),
        expires_at: model.expires_at.map(|t| t.to_rfc3339()),
        visit_count: model.visit_count,
        short_url: build_short_url(state.base_url.as_deref(), &code),
    };

    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn get_short_url(
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let model = state.short_url_service.find_by_code(&code).await?;
    ensure_not_expired(&model)?;

    let response = ShortUrlResponse {
        id: model.id,
        short_code: code,
        url: model.original_url,
        status: model.status,
        is_deleted: model.is_deleted,
        created_at: model.created_at.to_rfc3339(),
        updated_at: model.updated_at.to_rfc3339(),
        deleted_at: model.deleted_at.map(|t| t.to_rfc3339()),
        expires_at: model.expires_at.map(|t| t.to_rfc3339()),
        visit_count: model.visit_count,
        short_url: build_short_url(
            state.base_url.as_deref(),
            &model.short_code.clone().unwrap_or_default(),
        ),
    };

    Ok(Json(response))
}

pub async fn redirect_short_url(
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let model = state.short_url_service.find_by_code(&code).await?;
    ensure_not_expired(&model)?;

    state
        .short_url_service
        .increment_visit_count(model.id)
        .await?;
    Ok(Redirect::temporary(&model.original_url))
}

fn build_short_url(base_url: Option<&str>, code: &str) -> Option<String> {
    let base = base_url?.trim_end_matches('/');
    if base.is_empty() || code.is_empty() {
        return None;
    }
    Some(format!("{}/{}", base, code))
}

fn parse_expires_at(input: Option<String>) -> Result<Option<sea_orm::prelude::DateTimeWithTimeZone>, AppError> {
    let Some(value) = input else {
        return Ok(None);
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    let parsed = chrono::DateTime::parse_from_rfc3339(trimmed).map_err(|_| {
        AppError::bad_request_with_errors(
            "expires_at is invalid",
            ValidationErrors::single("expiresAt", "INVALID_FORMAT", "expires_at must be RFC3339"),
        )
    })?;

    Ok(Some(parsed))
}

fn ensure_not_expired(model: &crate::models::short_url::Model) -> Result<(), AppError> {
    if let Some(expires_at) = model.expires_at {
        if expires_at <= chrono::Utc::now().fixed_offset() {
            return Err(AppError::gone("short url expired"));
        }
    }
    Ok(())
}
