use crate::services::short_url_service::ShortUrlService;

#[derive(Clone)]
pub struct AppState {
    pub short_url_service: ShortUrlService,
    pub base_url: Option<String>,
}
