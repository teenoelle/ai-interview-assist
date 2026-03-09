use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::Response,
};
use futures::StreamExt;
use crate::state::AppState;

pub async fn ws_audio(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| handle_audio(socket, state.audio_tx))
}

pub async fn ws_audio_mic(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| handle_audio(socket, state.mic_audio_tx))
}

pub async fn ws_video(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| handle_video(socket, state.video_tx))
}

pub async fn ws_events(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| handle_events(socket, state.event_tx))
}

async fn handle_audio(mut socket: WebSocket, tx: tokio::sync::mpsc::Sender<Vec<u8>>) {
    while let Some(Ok(msg)) = socket.next().await {
        if let Message::Binary(data) = msg {
            let _ = tx.send(data.to_vec()).await;
        }
    }
}

async fn handle_video(mut socket: WebSocket, tx: tokio::sync::mpsc::Sender<Vec<u8>>) {
    while let Some(Ok(msg)) = socket.next().await {
        if let Message::Binary(data) = msg {
            let _ = tx.send(data.to_vec()).await;
        }
    }
}

async fn handle_events(mut socket: WebSocket, event_tx: tokio::sync::broadcast::Sender<common::messages::WsEvent>) {
    let mut rx = event_tx.subscribe();
    loop {
        match rx.recv().await {
            Ok(event) => {
                if let Ok(json) = serde_json::to_string(&event) {
                    if socket.send(Message::Text(json.into())).await.is_err() {
                        break;
                    }
                }
            }
            Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
        }
    }
}
