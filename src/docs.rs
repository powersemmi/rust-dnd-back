use crate::handlers::Params;
use crate::handlers::events::ClientEvent;

/// WebSocket Protocol Definition
///
/// Этот эндпоинт предназначен для WebSocket соединения.
/// Он здесь только для документации формата сообщений.
///
/// **Client -> Server (Commands):**
/// Отправляй JSON в таком формате в открытый сокет.
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
                        "type": "MOUSE_MOVE_TOKEN",
                        "data": {
                            "x": 150,
                            "y": 240,
                            "user_id": "hero_aragorn_1"
                        }
                    })
                )
            ),
            (
                "Chat Message" = (
                    summary = "Сообщение в чат",
                    description = "Текстовое сообщение всем участникам",
                    value = json!({
                        "type": "CHAT_MESSAGE",
                        "data": {
                            "text": "Я кидаю инициативу!",
                            "user_id": "hero_aragorn_1"
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
