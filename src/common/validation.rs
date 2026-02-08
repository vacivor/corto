use url::Url;

use crate::common::error::{AppError, ValidationErrors};

const CODE_REQUIRED: &str = "REQUIRED";
const CODE_INVALID_FORMAT: &str = "INVALID_FORMAT";
const CODE_INVALID_SCHEME: &str = "INVALID_SCHEME";

pub fn validate_url(input: &str, field_name: &str) -> Result<(), AppError> {
    if input.trim().is_empty() {
        return Err(AppError::bad_request_with_errors(
            format!("{} is required", field_name),
            ValidationErrors::single(
                field_name,
                CODE_REQUIRED,
                format!("{} is required", field_name),
            ),
        ));
    }

    let parsed = Url::parse(input).map_err(|_| {
        AppError::bad_request_with_errors(
            format!("{} is invalid", field_name),
            ValidationErrors::single(
                field_name,
                CODE_INVALID_FORMAT,
                format!("{} is invalid", field_name),
            ),
        )
    })?;

    match parsed.scheme() {
        "http" | "https" => Ok(()),
        _ => Err(AppError::bad_request_with_errors(
            format!("{} scheme must be http or https", field_name),
            ValidationErrors::single(
                field_name,
                CODE_INVALID_SCHEME,
                format!("{} scheme must be http or https", field_name),
            ),
        )),
    }
}
