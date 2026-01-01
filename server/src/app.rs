use axum::{
    http::Method,
    routing::{get, post},
    Router,
};
use tower_http::cors::{Any, CorsLayer};

use crate::db::AppState;
use crate::handlers::{
    delete_template, expand_character, expand_worldview, generate, generate_prompt,
    get_shared_game, get_shared_record_meta, hello, list_records, share_game, update_template,
};

pub(crate) fn build_app(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers(Any);

    Router::new()
        .route("/", get(hello))
        .route("/generate", post(generate))
        .route("/generate/prompt", post(generate_prompt))
        .route("/expand/worldview", post(expand_worldview))
        .route("/expand/character", post(expand_character))
        .route("/share", post(share_game))
        .route("/template/update", post(update_template))
        .route("/template/delete", post(delete_template))
        .route("/play/:id", get(get_shared_game))
        .route("/records", post(list_records))
        .route("/records/meta/:id", get(get_shared_record_meta))
        .with_state(state)
        .layer(cors)
}
