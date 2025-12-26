use crate::state::AppState;
use axum::extract::ws::{Message, Utf8Bytes, WebSocket};
use axum::extract::{Query, State, WebSocketUpgrade};
use axum::response::IntoResponse;
use futures::{SinkExt, StreamExt};
use shared::events::{ClientEvent, Params};
use std::string::String;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info};

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<Params>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    info!("Handling WebSocket connection for room: {}", params.room_id);
    ws.on_upgrade(move |socket| handle_socket(socket, params.room_id, state))
}

fn try_parse_event(text: &str) -> Result<ClientEvent, Utf8Bytes> {
    // 1. Попытка распарсить JSON
    let event = match serde_json::from_str::<ClientEvent>(&text) {
        Ok(event) => event,
        Err(err) => {
            return Err(format!("{{\"error\": \"Invalid JSON: {}\"}}", err).into());
        }
    };
    if let Err(err) = event.validate() {
        return Err(format!("{{\"error\": \"Validation failed: {}\"}}", err).into());
    }
    Ok(event)
}

#[repr(u8)]
enum EventType {
    Text = 0,
    Redis = 1,
    Error = 2,
}

async fn handle_events(
    text: String,
    channel_name: &String,
    state: &Arc<AppState>,
) -> (Option<Utf8Bytes>, EventType) {
    let result = try_parse_event(&text);

    let mut redis_connect = state
        .get_redis()
        .get_multiplexed_async_connection()
        .await
        .unwrap();

    match result {
        Ok(event) => {
            match event {
                ClientEvent::Ping => (Some("{\"type\": \"PONG\"}".into()), EventType::Text),
                _ => {
                    let _: () = redis::cmd("PUBLISH")
                        .arg(&channel_name)
                        .arg(text.to_string()) // Отправляем проверенный JSON всем остальным
                        .query_async(&mut redis_connect)
                        .await
                        .unwrap();
                    (None, EventType::Redis)
                }
            }
        }
        Err(e) => (Some(e), EventType::Error),
    }
}

async fn handle_socket(socket: WebSocket, room_id: String, state: Arc<AppState>) {
    let channel_name = format!("room:{}", room_id);
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::channel::<Message>(100);
    let mut pubsub = state
        .get_redis()
        .get_async_pubsub()
        .await
        .expect("Failed to get pubsub connection");

    info!("User connected to channel with name {channel_name}");

    pubsub
        .subscribe(&channel_name)
        .await
        .expect("Failed to subscribe to channel");

    let mut write_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    let tx_send = tx.clone();
    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = pubsub.on_message().next().await {
            let payload: String = msg.get_payload().unwrap();
            debug!("Received message from Redis: {}", payload);

            if tx_send
                .send(Message::Text(Utf8Bytes::from(payload)))
                .await
                .is_err()
            {
                debug!("WebSocket connection closed");
                break;
            }
        }
    });

    let tx_recv = tx.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    let (event_msg, event_type) =
                        handle_events(text.to_string(), &channel_name, &state).await;
                    match (event_msg, event_type) {
                        (Some(resp), EventType::Text) => {
                            tx_recv.send(Message::Text(resp)).await.unwrap();
                            debug!("Text event received");
                        }
                        (Some(e), EventType::Error) => {
                            tx_recv.send(Message::Text(e)).await.unwrap();
                            debug!("Error event received");
                        }
                        (None, EventType::Redis) => {
                            debug!("Redis event received");
                        }
                        _ => {
                            error!("Unknown event type received")
                        }
                    }
                }
                Message::Close(_) => break,
                _ => {
                    error!("Unexpected message received: {msg:?}")
                }
            }
        }
    });

    tokio::select! {
        _ = (&mut send_task) => send_task.abort(),
        _ = (&mut recv_task) => recv_task.abort(),
        _ = (&mut write_task) => write_task.abort(),
    }

    info!("User disconnected from room {}", room_id);
}
