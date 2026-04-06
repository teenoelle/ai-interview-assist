use axum::{
    extract::{
        ws::{Message, WebSocket},
        Query, State, WebSocketUpgrade,
    },
    http::StatusCode,
    response::{IntoResponse, Response},
};
use futures::StreamExt;
use serde::Deserialize;
use crate::state::AppState;

#[derive(Deserialize, Default)]
pub struct TokenQuery {
    token: Option<String>,
}

fn check_token(state: &AppState, query_token: Option<&str>) -> bool {
    match &state.app_token {
        None => true, // no auth configured
        Some(expected) => query_token.map(|t| t == expected).unwrap_or(false),
    }
}

pub async fn ws_audio(ws: WebSocketUpgrade, Query(q): Query<TokenQuery>, State(state): State<AppState>) -> Response {
    if !check_token(&state, q.token.as_deref()) {
        return (StatusCode::UNAUTHORIZED, "Invalid or missing token").into_response();
    }
    ws.on_upgrade(move |socket| handle_audio(socket, state.audio_tx))
}

pub async fn ws_audio_mic(ws: WebSocketUpgrade, Query(q): Query<TokenQuery>, State(state): State<AppState>) -> Response {
    if !check_token(&state, q.token.as_deref()) {
        return (StatusCode::UNAUTHORIZED, "Invalid or missing token").into_response();
    }
    ws.on_upgrade(move |socket| handle_audio(socket, state.mic_audio_tx))
}

pub async fn ws_video(ws: WebSocketUpgrade, Query(q): Query<TokenQuery>, State(state): State<AppState>) -> Response {
    if !check_token(&state, q.token.as_deref()) {
        return (StatusCode::UNAUTHORIZED, "Invalid or missing token").into_response();
    }
    ws.on_upgrade(move |socket| handle_video(socket, state.video_tx))
}

pub async fn ws_events(ws: WebSocketUpgrade, Query(q): Query<TokenQuery>, State(state): State<AppState>) -> Response {
    if !check_token(&state, q.token.as_deref()) {
        return (StatusCode::UNAUTHORIZED, "Invalid or missing token").into_response();
    }
    ws.on_upgrade(move |socket| handle_events(socket, state.event_tx))
}

async fn handle_audio(mut socket: WebSocket, tx: tokio::sync::mpsc::Sender<Vec<u8>>) {
    let mut count: u64 = 0;
    while let Some(Ok(msg)) = socket.next().await {
        if let Message::Binary(data) = msg {
            count += 1;
            if count == 1 { tracing::info!("audio WS: first chunk ({} bytes)", data.len()); }
            // try_send never blocks — if the transcription channel is full, drop the chunk
            // rather than stalling the WS handler (which would let Chrome's keepalive time out).
            let _ = tx.try_send(data.to_vec());
        }
    }
    tracing::warn!("audio WS: closed after {} chunks", count);
}

async fn handle_video(mut socket: WebSocket, tx: tokio::sync::mpsc::Sender<Vec<u8>>) {
    while let Some(Ok(msg)) = socket.next().await {
        if let Message::Binary(data) = msg {
            let _ = tx.try_send(data.to_vec());
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
