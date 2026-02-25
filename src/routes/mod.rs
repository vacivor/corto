use axum::{
    Router,
    routing::{get, post},
};
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};

use crate::{
    app::AppState,
    handlers::{admin_short_url_handler, short_url_handler, preview_handler},
};

pub fn routes(state: AppState) -> Router {
    let static_dir = std::env::var("CORTO_STATIC_DIR")
        .unwrap_or_else(|_| "frontend/dist/corto-frontend/browser".to_string());
    let index_path = format!("{}/index.html", static_dir);

    let api = Router::new()
        .route("/api/short-urls", post(short_url_handler::create_short_url))
        .route(
            "/api/short-urls/{code}",
            get(short_url_handler::get_short_url),
        )
        .route("/api/preview", get(preview_handler::preview))
        .route("/r/{code}", get(short_url_handler::redirect_short_url))
        .route(
            "/api/admin/short-urls",
            get(admin_short_url_handler::list_short_urls),
        )
        .route(
            "/api/admin/short-urls/stats",
            get(admin_short_url_handler::short_url_stats),
        )
        .route(
            "/api/admin/short-urls/{id}",
            get(admin_short_url_handler::get_short_url)
                .patch(admin_short_url_handler::update_short_url)
                .delete(admin_short_url_handler::delete_short_url),
        );

    let spa = ServeDir::new(static_dir).not_found_service(ServeFile::new(index_path.clone()));

    Router::new()
        .merge(api)
        .fallback_service(spa)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
