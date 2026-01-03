use axum::{
    extract::{ws::WebSocket, State, WebSocketUpgrade},
    response::Response,
};
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::broadcast;

/// Convert database TimelineEvent to API TimelineEvent
fn convert_event(db_event: &crate::models::TimelineEvent) -> crate::TimelineEvent {
    use crate::models::EventType;
    use crate::TimelineEventType;

    let event_type = match db_event.event_type {
        EventType::NewRelease => TimelineEventType::NewRelease,
        EventType::SecurityAlert => TimelineEventType::SecurityAlert,
        EventType::PackageAdded => TimelineEventType::PackageAdded,
        EventType::PackageUpdated => TimelineEventType::PackageUpdated,
    };

    crate::TimelineEvent {
        id: db_event.id,
        event_type,
        package_name: db_event.package_name.clone(),
        version: db_event.version.clone(),
        message: db_event.description.clone(),
        metadata: None,
        created_at: db_event.created_at,
    }
}

/// Broadcaster for timeline events
#[derive(Clone)]
pub struct TimelineBroadcaster {
    tx: broadcast::Sender<crate::models::TimelineEvent>,
}

impl TimelineBroadcaster {
    pub fn new() -> Self {
        let (tx, _rx) = broadcast::channel(100);
        Self { tx }
    }

    /// Broadcast a timeline event to all connected clients
    pub fn broadcast(&self, event: crate::models::TimelineEvent) {
        // Ignore send errors - they just mean no receivers are listening
        let _ = self.tx.send(event);
    }

    /// Subscribe to timeline events
    fn subscribe(&self) -> broadcast::Receiver<crate::models::TimelineEvent> {
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
    tracing::debug!("New WebSocket connection established");
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
                && let Ok(ws_msg) = serde_json::from_str::<crate::WebSocketMessage>(&text)
            {
                match ws_msg {
                    crate::WebSocketMessage::Auth { token } => {
                        // Verify JWT and extract user_id
                        if let Ok(claims) = crate::auth::verify_jwt(&token)
                            && let Ok(uid) = claims.sub.parse::<u64>()
                        {
                            let _ = auth_tx.send(uid).await;
                        }
                    }
                    crate::WebSocketMessage::Ping => {
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
                Ok(db_event) = rx.recv() => {
                    // Filter events based on authentication:
                    // - If not authenticated: only send global events (user_id = None)
                    // - If authenticated: only send events for this user
                    let should_send = match (user_id, db_event.user_id) {
                        (None, None) => true,  // Not authenticated, global event
                        (Some(uid), Some(event_uid)) if uid == event_uid => true,  // Authenticated, personal event
                        _ => false,  // Don't send
                    };

                    if should_send {
                        // Convert database model to API type before sending
                        let api_event = convert_event(&db_event);
                        let msg = crate::WebSocketMessage::TimelineEvent { event: api_event };
                        let json = serde_json::to_string(&msg).unwrap();
                        if sender.send(axum::extract::ws::Message::Text(json.into())).await.is_err() {
                            break;
                        }
                    }
                }

                // Handle client authentication
                Some(uid) = auth_rx.recv() => {
                    user_id = Some(uid);
                    tracing::debug!("WebSocket authenticated user: {}", uid);
                    // Note: WebSocketMessage doesn't have an Authenticated variant,
                    // so we don't send a response. Client knows auth succeeded when they get personal events.
                }

                // Respond to client ping
                Some(()) = ping_rx.recv() => {
                    let msg = crate::WebSocketMessage::Pong;
                    let json = serde_json::to_string(&msg).unwrap();
                    if sender.send(axum::extract::ws::Message::Text(json.into())).await.is_err() {
                        break;
                    }
                }

                // Send periodic server-side pings for keepalive
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(30)) => {
                    let msg = crate::WebSocketMessage::Ping;
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
    tracing::debug!("WebSocket connection closed");
}
