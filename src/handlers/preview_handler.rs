use axum::{
    extract::Query,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::common::error::{AppError, ValidationErrors};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewQuery {
    pub url: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewResponse {
    pub url: String,
    pub title: Option<String>,
}

pub async fn preview(Query(query): Query<PreviewQuery>) -> Result<impl IntoResponse, AppError> {
    validate_url(&query.url)?;

    let title = fetch_title(&query.url).await?;

    Ok(Json(PreviewResponse {
        url: query.url,
        title,
    }))
}

fn validate_url(input: &str) -> Result<(), AppError> {
    if input.trim().is_empty() {
        return Err(AppError::bad_request_with_errors(
            "url is required",
            ValidationErrors::single("url", "REQUIRED", "url is required"),
        ));
    }

    let parsed = Url::parse(input).map_err(|_| {
        AppError::bad_request_with_errors(
            "url is invalid",
            ValidationErrors::single("url", "INVALID_FORMAT", "url is invalid"),
        )
    })?;

    match parsed.scheme() {
        "http" | "https" => Ok(()),
        _ => Err(AppError::bad_request_with_errors(
            "url scheme must be http or https",
            ValidationErrors::single("url", "INVALID_SCHEME", "url scheme must be http or https"),
        )),
    }
}

async fn fetch_title(target: &str) -> Result<Option<String>, AppError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(4))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .map_err(|err| AppError::internal(format!("failed to build http client: {err}")))?;

    let response = client
        .get(target)
        .send()
        .await
        .map_err(|err| AppError::internal(format!("failed to fetch url: {err}")))?;

    if !response.status().is_success() {
        return Ok(None);
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|err| AppError::internal(format!("failed to read response: {err}")))?;

    if bytes.len() > 512 * 1024 {
        return Ok(None);
    }

    let html = String::from_utf8_lossy(&bytes);
    let doc = scraper::Html::parse_document(&html);
    let selector = scraper::Selector::parse("title")
        .map_err(|_| AppError::internal("failed to parse html"))?;

    let title = doc
        .select(&selector)
        .next()
        .map(|node| node.text().collect::<String>().trim().to_string())
        .filter(|t| !t.is_empty());

    Ok(title)
}
