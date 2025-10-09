use axum::{routing::{post, get}, Json, Router};
use axum::extract::Query;

#[derive(serde::Deserialize)]
struct PostMessageReq { //TODO not strings here for sure
    from_pub: String,
    to_pub: String,
    payload: String,
}

async fn post_message(Json(req): Json<PostMessageReq>) -> Result<Json<()>, (axum::http::StatusCode, String)> {
    Ok(Json(()))
}

#[derive(serde::Deserialize)]
struct UpdatesQuery {
    recipient: String,
    #[serde(default)]
    since_ms: Option<u64>,
    #[serde(default)]
    limit: Option<usize>,
}

async fn get_updates(Query(query): Query<UpdatesQuery>) -> Result<Json<()>, (axum::http::StatusCode, String)> {
    Ok(Json(()))
}

pub async fn listen_api(bind: std::net::SocketAddr) {
    let app = Router::new()
        .route("/api/v1/message", post(post_message))
        .route("/api/v1/updates", get(get_updates));

    let listener = tokio::net::TcpListener::bind(bind).await.unwrap();
    axum::serve(listener, app).await.unwrap();

}