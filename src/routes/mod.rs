use axum::{routing::{get, post}, Router};
use tower_http::trace::TraceLayer;

use crate::{
    handlers::{short_url_handler, admin_short_url_handler},
    app::AppState,
};

pub fn routes(state: AppState) -> Router {
    Router::new()
        .route("/api/short-urls", post(short_url_handler::create_short_url))
        .route("/api/short-urls/{code}", get(short_url_handler::get_short_url))
        .route("/{code}", get(short_url_handler::redirect_short_url))
        .route("/admin/short-urls", get(admin_short_url_handler::list_short_urls))
        .route(
            "/admin/short-urls/{id}",
            get(admin_short_url_handler::get_short_url)
                .patch(admin_short_url_handler::update_short_url)
                .delete(admin_short_url_handler::delete_short_url),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
