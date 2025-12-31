use axum::{
    extract::{ws::WebSocket, State, WebSocketUpgrade},
    response::Response,
};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::models::TimelineEvent;

/// Message types sent over WebSocket
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsMessage {
    /// Server sends timeline events to clients
    TimelineEvent { event: TimelineEvent },
    /// Client sends authentication
    Authenticate { token: String },
    /// Server acknowledges authentication
    Authenticated { user_id: u64 },
    /// Ping/Pong for keepalive
    Ping,
    Pong,
}

/// Broadcaster for timeline events
#[derive(Clone)]
pub struct TimelineBroadcaster {
    tx: broadcast::Sender<TimelineEvent>,
}

impl TimelineBroadcaster {
    pub fn new() -> Self {
        let (tx, _rx) = broadcast::channel(100);
        Self { tx }
    }

    /// Broadcast a timeline event to all connected clients
    pub fn broadcast(&self, event: TimelineEvent) {
        // Ignore send errors - they just mean no receivers are listening
        let _ = self.tx.send(event);
    }

    /// Subscribe to timeline events
    fn subscribe(&self) -> broadcast::Receiver<TimelineEvent> {
        self.tx.subscribe()
    }
}

/// WebSocket handler for timeline updates
pub async fn timeline_websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<crate::AppState>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state.broadcaster))
}

async fn handle_socket(socket: WebSocket, broadcaster: Arc<TimelineBroadcaster>) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = broadcaster.subscribe();
    let mut user_id: Option<u64> = None;

    // Use channels to communicate from receiver to sender
    let (auth_tx, mut auth_rx) = tokio::sync::mpsc::channel::<u64>(1);
    let (ping_tx, mut ping_rx) = tokio::sync::mpsc::channel::<()>(1);

    // Spawn a task to receive messages from the client
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let axum::extract::ws::Message::Text(text) = msg
                && let Ok(ws_msg) = serde_json::from_str::<WsMessage>(&text)
            {
                match ws_msg {
                    WsMessage::Authenticate { token } => {
                        // Verify JWT and extract user_id
                        if let Ok(claims) = crate::auth::verify_jwt(&token)
                            && let Ok(uid) = claims.sub.parse::<u64>()
                        {
                            let _ = auth_tx.send(uid).await;
                        }
                    }
                    WsMessage::Ping => {
                        // Notify send task to respond with Pong
                        let _ = ping_tx.send(()).await;
                    }
                    _ => {}
                }
            }
        }
    });

    // Spawn a task to send messages to the client
    let mut send_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                // Receive timeline events from the broadcaster
                Ok(event) = rx.recv() => {
                    // Filter events based on authentication:
                    // - If not authenticated: only send global events (user_id = None)
                    // - If authenticated: only send events for this user
                    let should_send = match (user_id, event.user_id) {
                        (None, None) => true,  // Not authenticated, global event
                        (Some(uid), Some(event_uid)) if uid == event_uid => true,  // Authenticated, personal event
                        _ => false,  // Don't send
                    };

                    if should_send {
                        let msg = WsMessage::TimelineEvent { event };
                        let json = serde_json::to_string(&msg).unwrap();
                        if sender.send(axum::extract::ws::Message::Text(json.into())).await.is_err() {
                            break;
                        }
                    }
                }

                // Handle client authentication
                Some(uid) = auth_rx.recv() => {
                    user_id = Some(uid);
                    let msg = WsMessage::Authenticated { user_id: uid };
                    let json = serde_json::to_string(&msg).unwrap();
                    if sender.send(axum::extract::ws::Message::Text(json.into())).await.is_err() {
                        break;
                    }
                }

                // Respond to client ping
                Some(()) = ping_rx.recv() => {
                    let msg = WsMessage::Pong;
                    let json = serde_json::to_string(&msg).unwrap();
                    if sender.send(axum::extract::ws::Message::Text(json.into())).await.is_err() {
                        break;
                    }
                }

                // Send periodic server-side pings for keepalive
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(30)) => {
                    let msg = WsMessage::Ping;
                    let json = serde_json::to_string(&msg).unwrap();
                    if sender.send(axum::extract::ws::Message::Text(json.into())).await.is_err() {
                        break;
                    }
                }
            }
        }
    });

    // Wait for either task to finish (which means the connection is closed)
    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    }
}
