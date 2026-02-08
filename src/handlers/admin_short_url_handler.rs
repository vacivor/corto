use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
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
pub struct ListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<i16>,
    pub is_deleted: Option<i16>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRequest {
    pub original_url: Option<String>,
    pub status: Option<i16>,
    pub is_deleted: Option<i16>,
    pub expires_at: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortUrlAdminResponse {
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
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListResponse {
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
    pub items: Vec<ShortUrlAdminResponse>,
}

pub async fn list_short_urls(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> Result<impl IntoResponse, AppError> {
    let page_size = query.page_size.unwrap_or(20).min(100);
    let page = query.page.unwrap_or(1).max(1);
    let offset = (page - 1) * page_size;

    let (total, models) = state
        .short_url_service
        .list_short_urls(page_size, offset, query.status, query.is_deleted)
        .await?;

    let items = models
        .into_iter()
        .map(|model| ShortUrlAdminResponse {
            id: model.id,
            short_code: model.short_code.unwrap_or_default(),
            url: model.original_url,
            status: model.status,
            is_deleted: model.is_deleted,
            created_at: model.created_at.to_rfc3339(),
            updated_at: model.updated_at.to_rfc3339(),
            deleted_at: model.deleted_at.map(|t| t.to_rfc3339()),
            expires_at: model.expires_at.map(|t| t.to_rfc3339()),
            visit_count: model.visit_count,
        })
        .collect();

    Ok(Json(ListResponse {
        total,
        page,
        page_size,
        items,
    }))
}

pub async fn get_short_url(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    let model = state.short_url_service.find_by_id(id).await?;

    Ok(Json(ShortUrlAdminResponse {
        id: model.id,
        short_code: model.short_code.unwrap_or_default(),
        url: model.original_url,
        status: model.status,
        is_deleted: model.is_deleted,
        created_at: model.created_at.to_rfc3339(),
        updated_at: model.updated_at.to_rfc3339(),
        deleted_at: model.deleted_at.map(|t| t.to_rfc3339()),
        expires_at: model.expires_at.map(|t| t.to_rfc3339()),
        visit_count: model.visit_count,
    }))
}

pub async fn update_short_url(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<UpdateRequest>,
) -> Result<impl IntoResponse, AppError> {
    let expires_at = parse_update_expires_at(payload.expires_at)?;

    if let Some(url) = payload.original_url.as_deref() {
        validate_url(url, "originalUrl")?;
    }

    let updated = state
        .short_url_service
        .update_short_url(
            id,
            payload.original_url,
            payload.status,
            payload.is_deleted,
            expires_at,
        )
        .await?;

    Ok((StatusCode::OK, Json(ShortUrlAdminResponse {
        id: updated.id,
        short_code: updated.short_code.unwrap_or_default(),
        url: updated.original_url,
        status: updated.status,
        is_deleted: updated.is_deleted,
        created_at: updated.created_at.to_rfc3339(),
        updated_at: updated.updated_at.to_rfc3339(),
        deleted_at: updated.deleted_at.map(|t| t.to_rfc3339()),
        expires_at: updated.expires_at.map(|t| t.to_rfc3339()),
        visit_count: updated.visit_count,
    })))
}

pub async fn delete_short_url(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    state.short_url_service.soft_delete(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

fn parse_update_expires_at(
    input: Option<String>,
) -> Result<Option<Option<sea_orm::prelude::DateTimeWithTimeZone>>, AppError> {
    let Some(value) = input else {
        return Ok(None);
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(Some(None));
    }

    let parsed = chrono::DateTime::parse_from_rfc3339(trimmed).map_err(|_| {
        AppError::bad_request_with_errors(
            "expires_at is invalid",
            ValidationErrors::single("expiresAt", "INVALID_FORMAT", "expires_at must be RFC3339"),
        )
    })?;

    Ok(Some(Some(parsed)))
}
