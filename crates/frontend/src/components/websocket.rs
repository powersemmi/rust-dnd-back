use futures::{SinkExt, StreamExt};
use gloo_net::websocket::{Message, futures::WebSocket};
use leptos::prelude::*;
use shared::events::{ChatMessagePayload, ClientEvent};
use std::collections::HashMap;
use wasm_bindgen_futures::spawn_local;

use crate::config;

pub type WsSender = futures::channel::mpsc::UnboundedSender<ClientEvent>;
pub type CursorSignals = (RwSignal<i32>, RwSignal<i32>);

pub fn connect_websocket(
    room_name: String,
    jwt_token: String,
    set_ws_sender: WriteSignal<Option<WsSender>>,
    set_cursors: WriteSignal<HashMap<String, CursorSignals>>,
    messages: RwSignal<Vec<ChatMessagePayload>>,
    config: config::Config,
) {
    spawn_local(async move {
        // Определяем протокол WebSocket на основе HTTP протокола
        let ws_protocol = if config.api.back_url.starts_with("https://") {
            "wss://"
        } else {
            "ws://"
        };

        // Убираем http:// или https:// из back_url
        let host = config.api.back_url
            .trim_start_matches("http://")
            .trim_start_matches("https://");

        let ws_url = format!(
            "{}{}{}?room_id={}&token={}",
            ws_protocol, host, config.api.ws_path, room_name, jwt_token
        );
        let ws = WebSocket::open(&ws_url).unwrap();
        let (mut write, mut read) = ws.split();

        // Создаем канал для отправки событий
        let (tx, mut rx) = futures::channel::mpsc::unbounded::<ClientEvent>();
        set_ws_sender.set(Some(tx));

        // Задача: Чтение из сокета -> Обновление UI
        spawn_local(async move {
            while let Some(msg) = read.next().await {
                if let Ok(Message::Text(text)) = msg {
                    if let Ok(event) = serde_json::from_str::<ClientEvent>(&text) {
                        match event {
                            ClientEvent::MouseClickPayload(p) => {
                                // Обновляем мапу курсоров
                                set_cursors.update(|map| {
                                    if let Some((sig_x, sig_y)) = map.get(&p.user_id) {
                                        sig_x.set(p.x);
                                        sig_y.set(p.y);
                                    } else {
                                        map.insert(
                                            p.user_id.clone(),
                                            (RwSignal::new(p.x), RwSignal::new(p.y)),
                                        );
                                    }
                                });
                            }
                            ClientEvent::ChatMessage(msg) => {
                                // Добавляем новое сообщение в список
                                messages.update(|msgs| {
                                    msgs.push(msg);
                                });
                            }
                            _ => {}
                        }
                    }
                }
            }
        });

        // Задача: UI (канал) -> Сокет
        spawn_local(async move {
            while let Some(event) = rx.next().await {
                let json = serde_json::to_string(&event).unwrap();
                let _ = write.send(Message::Text(json)).await;
            }
        });
    });
}
