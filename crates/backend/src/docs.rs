use shared::events::{ClientEvent, Params};

/// WebSocket Protocol Definition
///
/// Этот эндпоинт предназначен для WebSocket соединения.
/// Он здесь только для документации формата сообщений.
///
/// **Client -> Server (Commands):**
/// Отправляй JSON в таком формате в открытый сокет.
/// Для защищённых типов (`chat`, `note`, `file`, `sync snapshot`) plaintext-протокол отключён:
/// используются `CRYPTO_KEY_ANNOUNCE`, `CRYPTO_KEY_WRAP`, `CRYPTO_PAYLOAD`.
///
/// **Server -> Client (Events):**
/// Сервер будет присылать JSON в таком формате.
#[utoipa::path(
    post,
    path = "/ws/room",
    tag = "WebSocket Protocol",
    params(Params),
    request_body(
        content = ClientEvent,
        description = "Сообщения, которые отправляет Client",
        examples((
                "Move Token" = (
                    summary = "Перемещение мыши",
                    description = "Игрок двигает мышку по доске",
                    value = json!({
                        "type": "MOUSE_EVENT",
                        "data": {
                            "x": 150,
                            "y": 240,
                            "mouse_event_type": "Move",
                            "user_id": "hero_aragorn_1"
                        }
                    })
                )
            ),
            (
                "Encrypted Chat Payload" = (
                    summary = "Зашифрованное сообщение",
                    description = "Контент чата/заметок/файлов передаётся в CRYPTO_PAYLOAD",
                    value = json!({
                        "type": "CRYPTO_PAYLOAD",
                        "data": {
                            "version": 1,
                            "key_id": "3e462f13-3941-4e7b-b8c6-b0c684f2f8f2",
                            "sender_username": "hero_aragorn_1",
                            "kind": "CHAT",
                            "nonce_b64": "base64-nonce",
                            "ciphertext_b64": "base64-ciphertext"
                        }
                    })
                )
            ),
            (
                "Ping" = (
                    summary = "Проверка связи",
                    description = "Технический запрос для поддержания соединения",
                    value = json!({
                        "type": "PING",
                    })
                )
            ),
    )
    ),
    responses(
        (status = 200, description = "Сообщения, которые присылает Server (эвенты от других клиентов)", body = ClientEvent)
        // Если у тебя есть ServerEvent, используй его здесь вместо ClientEvent
    )
)]
pub async fn websocket_docs() {}
