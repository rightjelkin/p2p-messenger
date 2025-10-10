pub mod prelude;
mod command;

use axum::{routing::{post, get}, Json, Router};
use axum::extract::{Query, State};
use command::{Command, Envelope, ExecResult};

#[derive(Clone)]
struct ApiState {
    cmd_tx: tokio::sync::mpsc::Sender<Command>
}

#[derive(serde::Deserialize)]
struct PostMessageReq {
    from_pub: String,
    to_pub: String,
    payload: String,
}

async fn post_message(State(state): State<ApiState>, Json(req): Json<PostMessageReq>) -> Result<Json<ExecResult>, (axum::http::StatusCode, String)> {
    //TODO validate pubkeys
    //TODO store
    let mut envelope = Envelope {
        msg_id: [0u8; 32],
        from_pub: req.from_pub.into(),
        to_pub: req.to_pub.into(),
        payload: req.payload.into(),
    };
    envelope.msg_id = command::compute_msg_id(&envelope);
    let (tx, rx) = tokio::sync::oneshot::channel();
    state.cmd_tx.send(Command::Submit { envelope, reply: tx })
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("send cmd: {e}")))?;
    let res = rx.await.map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("await: {e}")))?;
    Ok(Json(res))
}

#[derive(serde::Deserialize)]
struct UpdatesQuery {
    recipient: String,
    #[serde(default)]
    since_ms: Option<u64>,
    #[serde(default)]
    limit: Option<usize>,
}

async fn get_updates(State(state): State<ApiState>, Query(query): Query<UpdatesQuery>) -> Result<Json<Vec<Envelope>>, (axum::http::StatusCode, String)> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    state.cmd_tx.send(Command::FetchLocal { recipient: query.recipient.clone().into(), since_ms: query.since_ms.unwrap_or(0), limit: query.limit.unwrap_or(100), reply: tx })
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("send cmd: {e}")))?;
    let res = rx.await.map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("await: {e}")))?;
    Ok(Json(res))
}

pub async fn listen_api(cmd_tx: tokio::sync::mpsc::Sender<Command>, bind: std::net::SocketAddr) {
    let state = ApiState { cmd_tx };
    let app = Router::new()
        .route("/api/v1/message", post(post_message))
        .route("/api/v1/updates", get(get_updates))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind(bind).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}