
mod vitals;
mod ws;

use axum::{
    routing::get,
    Router,
};
use tower_http::cors::CorsLayer;
use tracing::info;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/", get(serve_index))
        .route("/ws", get(ws::ws_handler))
        .layer(CorsLayer::permissive());

    let addr = "0.0.0.0:3000";
    info!("Pi Vitals dashboard running on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn serve_index() -> axum::response::Html<&'static str> {
    axum::response::Html(include_str!("index.html"))
}