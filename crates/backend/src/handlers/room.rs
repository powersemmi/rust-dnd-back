use crate::AppError;
use crate::AppState;
use crate::utils::jwt::verify_jwt;
use crate::ws_policy::{
    ConnectionRateLimiter, IncomingMessageKind, MAX_INBOUND_MESSAGE_SIZE_BYTES,
    close_frame_for_violation,
};
use axum::extract::ws::{Message, Utf8Bytes, WebSocket};
use axum::extract::{Query, State, WebSocketUpgrade};
use axum::response::{IntoResponse, Response};
use futures::{
    SinkExt, StreamExt,
    stream::{SplitSink, SplitStream},
};
use redis::aio::PubSub;
use serde_json::json;
use shared::events::{ClientEvent, EncryptedPayloadKind, Params};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{debug, error, info};
use uuid::Uuid;

const ROOM_ACTIVITY_TTL_SECONDS: u64 = 86_400;
const WEBSOCKET_CHANNEL_CAPACITY: usize = 100;

type SocketSender = SplitSink<WebSocket, Message>;
type SocketReceiver = SplitStream<WebSocket>;

fn try_parse_event(text: &str) -> Result<ClientEvent, Utf8Bytes> {
    let event = serde_json::from_str::<ClientEvent>(text).map_err(|error| {
        Utf8Bytes::from(json!({ "error": format!("Invalid JSON: {error}") }).to_string())
    })?;

    if let Err(error) = event.validate() {
        return Err(Utf8Bytes::from(
            json!({ "error": format!("Validation failed: {error}") }).to_string(),
        ));
    }

    Ok(event)
}

async fn process_client_message(
    text: String,
    channel_name: &str,
    state: &AppState,
) -> Option<Utf8Bytes> {
    let event = match try_parse_event(&text) {
        Ok(event) => event,
        Err(error) => return Some(error),
    };

    if is_plaintext_legacy_event(&event) {
        return Some(
            json!({ "error": "Plaintext protocol is disabled for this event type" })
                .to_string()
                .into(),
        );
    }

    match event {
        ClientEvent::Ping => Some(json!({ "type": "PONG" }).to_string().into()),
        _ => {
            if let Err(error) = publish_event(&state.redis, channel_name, &text).await {
                error!(
                    "Failed to publish message to Redis for channel {}: {}",
                    channel_name, error
                );
                return Some(
                    json!({ "error": "Failed to broadcast message" })
                        .to_string()
                        .into(),
                );
            }

            if let Err(error) = refresh_room_activity(&state.redis, channel_name).await {
                error!(
                    "Failed to refresh room activity for channel {}: {}",
                    channel_name, error
                );
            }

            None
        }
    }
}

fn is_plaintext_legacy_event(event: &ClientEvent) -> bool {
    matches!(
        event,
        ClientEvent::ChatMessage(_)
            | ClientEvent::NoteUpsert(_)
            | ClientEvent::NoteDelete(_)
            | ClientEvent::FileAnnounce(_)
            | ClientEvent::FileRequest(_)
            | ClientEvent::FileChunk(_)
            | ClientEvent::FileAbort(_)
            | ClientEvent::SyncSnapshot(_)
            | ClientEvent::DirectMessage(_)
    )
}

async fn publish_event(
    redis: &redis::Client,
    channel_name: &str,
    payload: &str,
) -> redis::RedisResult<()> {
    let mut connection = redis.get_multiplexed_async_connection().await?;
    redis::cmd("PUBLISH")
        .arg(channel_name)
        .arg(payload)
        .query_async::<usize>(&mut connection)
        .await?;

    Ok(())
}

async fn refresh_room_activity(
    redis: &redis::Client,
    channel_name: &str,
) -> redis::RedisResult<()> {
    let mut connection = redis.get_multiplexed_async_connection().await?;
    let activity_key = activity_key(channel_name);

    redis::cmd("SETEX")
        .arg(&activity_key)
        .arg(ROOM_ACTIVITY_TTL_SECONDS)
        .arg("1")
        .query_async::<()>(&mut connection)
        .await?;

    Ok(())
}

async fn ensure_room_activity(
    redis: &redis::Client,
    room_id: &str,
    channel_name: &str,
) -> redis::RedisResult<()> {
    let mut connection = redis.get_multiplexed_async_connection().await?;
    let activity_key = activity_key(channel_name);
    let exists = redis::cmd("EXISTS")
        .arg(&activity_key)
        .query_async::<bool>(&mut connection)
        .await?;

    if !exists {
        debug!("Room {} was inactive, resetting activity", room_id);
        redis::cmd("SETEX")
            .arg(&activity_key)
            .arg(ROOM_ACTIVITY_TTL_SECONDS)
            .arg("1")
            .query_async::<()>(&mut connection)
            .await?;
    }

    Ok(())
}

fn activity_key(channel_name: &str) -> String {
    format!("{channel_name}:activity")
}

fn spawn_send_task(mut sender: SocketSender, mut rx: mpsc::Receiver<Message>) -> JoinHandle<()> {
    tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            if let Err(error) = sender.send(message).await {
                debug!("Failed to send WebSocket message: {}", error);
                break;
            }
        }
    })
}

fn spawn_redis_listener(mut pubsub: PubSub, tx: mpsc::Sender<Message>) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut messages = pubsub.on_message();

        while let Some(message) = messages.next().await {
            let payload = match message.get_payload::<String>() {
                Ok(payload) => payload,
                Err(error) => {
                    error!("Failed to decode Redis payload: {}", error);
                    continue;
                }
            };

            debug!("Received message from Redis: {}", payload);
            if tx.send(Message::Text(payload.into())).await.is_err() {
                debug!("WebSocket connection closed while delivering Redis payload");
                break;
            }
        }
    })
}

fn classify_incoming_message(event: &ClientEvent) -> IncomingMessageKind {
    match event {
        ClientEvent::MouseClickPayload(_) => IncomingMessageKind::Mouse,
        ClientEvent::FileChunk(_) => IncomingMessageKind::FileChunk,
        ClientEvent::CryptoPayload(payload) => match payload.kind {
            EncryptedPayloadKind::FileChunk => IncomingMessageKind::FileChunk,
            _ => IncomingMessageKind::General,
        },
        _ => IncomingMessageKind::General,
    }
}

fn spawn_receive_task(
    mut receiver: SocketReceiver,
    tx: mpsc::Sender<Message>,
    channel_name: String,
    state: Arc<AppState>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut rate_limiter = ConnectionRateLimiter::default();

        while let Some(message) = receiver.next().await {
            match message {
                Ok(Message::Text(text)) => {
                    let parsed_event = serde_json::from_str::<ClientEvent>(&text);
                    let message_kind = parsed_event
                        .as_ref()
                        .map(classify_incoming_message)
                        .unwrap_or(IncomingMessageKind::General);

                    if let Err(violation) = rate_limiter.check(message_kind) {
                        let close_frame = close_frame_for_violation(violation);
                        info!(
                            "Closing room channel {} due to websocket rate limit: {}",
                            channel_name, close_frame.reason
                        );

                        let _ = tx.send(Message::Close(Some(close_frame))).await;
                        break;
                    }

                    if let Some(response) =
                        process_client_message(text.to_string(), &channel_name, state.as_ref())
                            .await
                        && tx.send(Message::Text(response)).await.is_err()
                    {
                        debug!("WebSocket connection closed while returning handler response");
                        break;
                    }
                }
                Ok(Message::Close(_)) => break,
                Ok(other) => error!("Unexpected message received: {:?}", other),
                Err(error) => {
                    error!("Failed to read WebSocket message: {}", error);
                    break;
                }
            }
        }
    })
}

async fn handle_socket(socket: WebSocket, room_id: String, user_id: Uuid, state: Arc<AppState>) {
    let channel_name = format!("room:{room_id}");
    let (sender, receiver) = socket.split();
    let (tx, rx) = mpsc::channel::<Message>(WEBSOCKET_CHANNEL_CAPACITY);

    if let Err(error) = ensure_room_activity(&state.redis, &room_id, &channel_name).await {
        error!(
            "Failed to check or update room activity for room {}: {}",
            room_id, error
        );
        return;
    }

    let mut pubsub = match state.redis.get_async_pubsub().await {
        Ok(pubsub) => pubsub,
        Err(error) => {
            error!("Failed to get Redis pubsub connection: {}", error);
            return;
        }
    };

    info!(
        "User={} connected to channel with channel_name={}",
        user_id, channel_name
    );

    if let Err(error) = pubsub.subscribe(&channel_name).await {
        error!("Failed to subscribe to channel {}: {}", channel_name, error);
        return;
    }

    let mut send_task = spawn_send_task(sender, rx);
    let mut redis_task = spawn_redis_listener(pubsub, tx.clone());
    let mut receive_task = spawn_receive_task(receiver, tx, channel_name, state);

    tokio::select! {
        _ = &mut send_task => {
            redis_task.abort();
            receive_task.abort();
        }
        _ = &mut redis_task => {
            send_task.abort();
            receive_task.abort();
        }
        _ = &mut receive_task => {
            send_task.abort();
            redis_task.abort();
        }
    }

    info!("User disconnected from room {}", room_id);
}

pub async fn ws_room_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<Params>,
    State(state): State<Arc<AppState>>,
) -> Response {
    let claims = match verify_jwt(&params.token) {
        Ok(claims) => claims,
        Err(error) => {
            error!("WebSocket auth failed: {}", error);
            return AppError::unauthorized("Invalid token").into_response();
        }
    };

    info!("Handling WebSocket connection for room: {}", params.room_id);
    ws.max_message_size(MAX_INBOUND_MESSAGE_SIZE_BYTES)
        .max_frame_size(MAX_INBOUND_MESSAGE_SIZE_BYTES)
        .on_upgrade(move |socket| handle_socket(socket, params.room_id, claims.sub, state))
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::events::{
        ChatMessagePayload, CryptoPayload, FileChunkPayload, SyncSnapshotPackedStatePayload,
        SyncSnapshotPayload,
    };

    #[test]
    fn plaintext_legacy_events_are_detected() {
        let chat = ClientEvent::ChatMessage(ChatMessagePayload {
            payload: "hello".to_string(),
            username: "alice".to_string(),
            attachments: Vec::new(),
        });
        assert!(is_plaintext_legacy_event(&chat));

        let encrypted = ClientEvent::CryptoPayload(CryptoPayload {
            version: 1,
            key_id: "k1".to_string(),
            sender_username: "alice".to_string(),
            kind: EncryptedPayloadKind::Chat,
            nonce_b64: "bm9uY2U=".to_string(),
            ciphertext_b64: "Y2lwaGVy".to_string(),
        });
        assert!(!is_plaintext_legacy_event(&encrypted));

        let snapshot = ClientEvent::SyncSnapshot(SyncSnapshotPayload {
            version: 1,
            packed_state: SyncSnapshotPackedStatePayload {
                codec_version: 1,
                compression: "gzip".to_string(),
                payload_b64: "cGF5bG9hZA==".to_string(),
            },
        });
        assert!(is_plaintext_legacy_event(&snapshot));
    }

    #[test]
    fn encrypted_file_chunk_is_rate_limited_as_file_chunk() {
        let encrypted_chunk = ClientEvent::CryptoPayload(CryptoPayload {
            version: 1,
            key_id: "k1".to_string(),
            sender_username: "alice".to_string(),
            kind: EncryptedPayloadKind::FileChunk,
            nonce_b64: "bm9uY2U=".to_string(),
            ciphertext_b64: "Y2lwaGVy".to_string(),
        });
        assert_eq!(
            classify_incoming_message(&encrypted_chunk),
            IncomingMessageKind::FileChunk
        );

        let plaintext_chunk = ClientEvent::FileChunk(FileChunkPayload {
            hash: "h".to_string(),
            requester: "alice".to_string(),
            chunk_index: 0,
            total_chunks: 1,
            data: "x".to_string(),
        });
        assert_eq!(
            classify_incoming_message(&plaintext_chunk),
            IncomingMessageKind::FileChunk
        );
    }
}
