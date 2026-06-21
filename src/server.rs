// Tiny HTTP/WS control server. Browser opens `/`, talks to `/ws`, and writes
// the current semitone target into the shared atomic. The audio thread reads
// it lock-free on every block.

use anyhow::Result;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use tower_http::services::ServeDir;

#[derive(Clone)]
struct AppState {
    semitones: Arc<AtomicI32>,
    sample_rate: u32,
    buffer_size: u32,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum ClientMsg {
    Set { semitones: i32 },
    Ping,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum ServerMsg {
    Hello {
        semitones: i32,
        sample_rate: u32,
        buffer_size: u32,
    },
    State {
        semitones: i32,
    },
    Pong,
}

pub async fn run(
    semitones: Arc<AtomicI32>,
    sample_rate: u32,
    buffer_size: u32,
    static_dir: PathBuf,
    addr: SocketAddr,
) -> Result<()> {
    let state = AppState {
        semitones,
        sample_rate,
        buffer_size,
    };

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .fallback_service(ServeDir::new(static_dir))
        .with_state(state);

    tracing::info!("HTTP server listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut tx, mut rx) = socket.split();

    let hello = serde_json::to_string(&ServerMsg::Hello {
        semitones: state.semitones.load(Ordering::Relaxed),
        sample_rate: state.sample_rate,
        buffer_size: state.buffer_size,
    })
    .expect("hello serialization");
    if tx.send(Message::Text(hello)).await.is_err() {
        return;
    }

    while let Some(Ok(msg)) = rx.next().await {
        match msg {
            Message::Text(text) => match serde_json::from_str::<ClientMsg>(&text) {
                Ok(ClientMsg::Set { semitones }) => {
                    let clamped = semitones.clamp(-12, 12);
                    state.semitones.store(clamped, Ordering::Relaxed);
                    let reply = serde_json::to_string(&ServerMsg::State {
                        semitones: clamped,
                    })
                    .unwrap();
                    if tx.send(Message::Text(reply)).await.is_err() {
                        break;
                    }
                }
                Ok(ClientMsg::Ping) => {
                    let reply = serde_json::to_string(&ServerMsg::Pong).unwrap();
                    if tx.send(Message::Text(reply)).await.is_err() {
                        break;
                    }
                }
                Err(e) => tracing::debug!("bad ws message: {e}"),
            },
            Message::Close(_) => break,
            _ => {}
        }
    }
}
