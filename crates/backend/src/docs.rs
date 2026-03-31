use shared::events::{ClientEvent, Params};

/// # WebSocket API
///
/// Точка входа для WebSocket-соединения виртуального стола.
///
/// ## Подключение
///
/// ```text
/// GET /ws/room?room_id=<room>&token=<jwt>
/// ```
///
/// JWT-токен передаётся как query-параметр (не в заголовке), потому что
/// браузерный WebSocket API не позволяет задавать произвольные заголовки.
///
/// ---
///
/// ## Шифрование
///
/// Часть событий **обязательно шифруется** (plaintext-вариант отвергается сервером
/// с сообщением об ошибке):
///
/// | Тип содержимого | Шифруется? | kind в CRYPTO_PAYLOAD |
/// |-----------------|------------|------------------------|
/// | `CHAT_MESSAGE`  | ✅ да       | `CHAT`                 |
/// | `DIRECT_MESSAGE`| ✅ да       | `CHAT`                 |
/// | `NOTE_UPSERT`, `NOTE_DELETE` | ✅ да | `NOTE`         |
/// | `SYNC_SNAPSHOT` | ✅ да       | `SYNC`                 |
/// | `FILE_ANNOUNCE`, `FILE_REQUEST`, `FILE_ABORT` | ✅ да | `FILE_CONTROL` |
/// | `FILE_CHUNK`    | ✅ да       | `FILE_CHUNK`           |
/// | Все остальные   | ❌ нет      | —                      |
///
/// ### Процесс обмена ключами (X25519 + ChaCha20-Poly1305)
///
/// 1. При подключении клиент отправляет **`CRYPTO_KEY_ANNOUNCE`** с публичным ключом.
/// 2. Каждый пир, получив объявление, оборачивает свой комнатный ключ через ECDH
///    и отправляет **`CRYPTO_KEY_WRAP`** адресату.
/// 3. Получив `CRYPTO_KEY_WRAP`, клиент может расшифровывать входящие
///    **`CRYPTO_PAYLOAD`** от этого отправителя.
/// 4. При отправке зашифрованного события клиент сначала высылает недостающие
///    `CRYPTO_KEY_WRAP` для новых пиров, затем сам **`CRYPTO_PAYLOAD`**.
///
/// ---
///
/// ## Лимиты
///
/// | Тип сообщения | Бюджет        |
/// |---------------|---------------|
/// | Общие         | 60 / 10 с     |
/// | Курсор мыши (`MOUSE_EVENT`) | 1200 / 10 с |
/// | Файловые чанки (`FILE_CHUNK`) | 1200 / 10 с |
///
/// При превышении лимита сервер закрывает WebSocket с кодом `policy-violation`.
///
/// ---
///
/// ## Полный список событий (Client → Server)
///
/// ### Присутствие / курсор
/// - **`MOUSE_EVENT`** — позиция курсора в мировых координатах.
/// - **`PRESENCE_REQUEST`** — запрос списка пользователей в комнате.
/// - **`PRESENCE_RESPONSE`** — ответ на `PRESENCE_REQUEST`.
/// - **`PRESENCE_ANNOUNCE`** — широковещательное объявление о присутствии.
///
/// ### Чат и личные сообщения
/// - **`CHAT_MESSAGE`** *(зашифровано)* — публичное сообщение в чат комнаты.
/// - **`DIRECT_MESSAGE`** *(зашифровано)* — личное сообщение конкретному
///   пользователю (синтаксис `@nick сообщение` в UI). Бэкенд ретранслирует
///   всем; только адресат обрабатывает содержимое.
///
/// ### Заметки
/// - **`NOTE_UPSERT`** *(зашифровано)* — создание / обновление заметки.
///   Видимость: `Public` (все), `Private` (только автор), `Direct(nick)` (личная).
/// - **`NOTE_DELETE`** *(зашифровано)* — удаление заметки.
///
/// ### Сцены и токены
/// - **`SCENE_CREATE`** — создание новой сцены.
/// - **`SCENE_UPDATE`** — обновление параметров сцены (фон, размер сетки, токены).
/// - **`SCENE_DELETE`** — удаление сцены.
/// - **`SCENE_ACTIVATE`** — переключение активной сцены для всей комнаты.
/// - **`TOKEN_MOVE`** — перемещение токена в мировых координатах.
///
/// ### Файлы
/// - **`FILE_ANNOUNCE`** *(зашифровано)* — объявление о доступном файле (по SHA-256 хешу).
/// - **`FILE_REQUEST`** *(зашифровано)* — запрос файла у объявившего пира.
/// - **`FILE_CHUNK`** *(зашифровано)* — чанк файла (base64, ~48 КБ). Детерминированная
///   маршрутизация: не все пиры отвечают на запрос, только один — по хешу.
/// - **`FILE_ABORT`** *(зашифровано)* — отмена передачи файла.
///
/// ### Голосование
/// - **`VOTING_START`** — начало голосования с вариантами ответов.
/// - **`VOTING_CAST`** — голос участника.
/// - **`VOTING_RESULT`** — публикация результатов (GM).
/// - **`VOTING_END`** — завершение голосования.
///
/// ### Синхронизация состояния
/// - **`SYNC_REQUEST`** — новый клиент запрашивает версии у всех пиров.
/// - **`SYNC_VERSION_ANNOUNCE`** — ответ: версия и хеш состояния пира.
/// - **`SYNC_SNAPSHOT_REQUEST`** — запрос полного снапшота у конкретного пира.
/// - **`SYNC_SNAPSHOT`** *(зашифровано)* — полный снапшот состояния комнаты
///   (сцены, чат, заметки, результаты голосований).
///
/// ### Инструменты доски
/// - **`BOARD_POINTER`** — включение / выключение режима указателя (один раз при
///   переключении; трейл строится получателем из потока `MOUSE_EVENT`).
/// - **`ATTENTION_PING`** — Alt+ЛКМ: пульсирующий пинг во всемировых координатах.
///
/// ### Крипто
/// - **`CRYPTO_KEY_ANNOUNCE`** — публичный ключ X25519 клиента.
/// - **`CRYPTO_KEY_WRAP`** — комнатный ключ, зашифрованный через ECDH для адресата.
/// - **`CRYPTO_PAYLOAD`** — зашифрованная полезная нагрузка (ChatMessage, Note и др.).
///
/// ### Служебные
/// - **`PING`** — keepalive; сервер отвечает `{"type":"PONG"}`.
#[utoipa::path(
    get,
    path = "/ws/room",
    tag = "WebSocket Protocol",
    params(Params),
    request_body(
        content = ClientEvent,
        description = "Сообщения клиента → сервер (ретранслируются всем участникам комнаты)",
        examples(
            // ── Presence ──────────────────────────────────────────────────────
            ("Mouse Move" = (
                summary = "MOUSE_EVENT — движение курсора",
                description = "Позиция мыши в мировых координатах. Высокочастотный поток; лимит 1200/10 с.",
                value = json!({
                    "type": "MOUSE_EVENT",
                    "data": { "x": 412.5, "y": 207.0, "mouse_event_type": "Move", "user_id": "aragorn" }
                })
            )),
            ("Presence Request" = (
                summary = "PRESENCE_REQUEST — запрос участников",
                value = json!({
                    "type": "PRESENCE_REQUEST",
                    "data": { "requester": "aragorn", "request_id": "req-001" }
                })
            )),
            // ── Chat ──────────────────────────────────────────────────────────
            ("Chat Message" = (
                summary = "CHAT_MESSAGE — публичный чат (шифруется)",
                description = "Отправляется только внутри CRYPTO_PAYLOAD с kind=CHAT.",
                value = json!({
                    "type": "CHAT_MESSAGE",
                    "data": {
                        "payload": "Привет всем!",
                        "username": "aragorn",
                        "attachments": []
                    }
                })
            )),
            ("Chat with Attachment" = (
                summary = "CHAT_MESSAGE — с вложением (шифруется)",
                description = "Файл сначала объявляется через FILE_ANNOUNCE, затем прикладывается сюда по хешу.",
                value = json!({
                    "type": "CHAT_MESSAGE",
                    "data": {
                        "payload": "Смотри карту",
                        "username": "aragorn",
                        "attachments": [{
                            "hash": "a3f9b2...c1",
                            "mime_type": "image/png",
                            "file_name": "map.png",
                            "size": 204800
                        }]
                    }
                })
            )),
            ("Direct Message" = (
                summary = "DIRECT_MESSAGE — личное сообщение (шифруется)",
                description = "Отправить можно через '@nick текст' в UI чата. Бэкенд ретранслирует всем; только получатель обрабатывает содержимое.",
                value = json!({
                    "type": "DIRECT_MESSAGE",
                    "data": {
                        "from": "aragorn",
                        "to": "legolas",
                        "body": "Встречаемся у ворот Мории",
                        "sent_at_ms": 1720000000000_u64
                    }
                })
            )),
            // ── Notes ─────────────────────────────────────────────────────────
            ("Note Upsert Public" = (
                summary = "NOTE_UPSERT — публичная заметка на доске (шифруется)",
                value = json!({
                    "type": "NOTE_UPSERT",
                    "data": {
                        "id": "note-uuid",
                        "author": "gm",
                        "visibility": "Public",
                        "title": "",
                        "body": "## Инициатива\n1. Арагорн\n2. Леголас",
                        "created_at_ms": 1720000000000_u64,
                        "updated_at_ms": 1720000001000_u64,
                        "board_position": { "world_x": 320.0, "world_y": 150.0 },
                        "board_style": { "width_px": 280.0, "height_px": 220.0, "font_size_pt": 14.0, "color": "#F8EE96" }
                    }
                })
            )),
            ("Note Upsert Direct" = (
                summary = "NOTE_UPSERT — личная заметка (шифруется)",
                description = "visibility: { Direct: 'nick' } — виден только автору и получателю.",
                value = json!({
                    "type": "NOTE_UPSERT",
                    "data": {
                        "id": "note-uuid-2",
                        "author": "gm",
                        "visibility": { "Direct": "aragorn" },
                        "title": "",
                        "body": "Твой персонаж знает о предательстве.",
                        "created_at_ms": 1720000002000_u64,
                        "updated_at_ms": 1720000002000_u64
                    }
                })
            )),
            ("Note Delete" = (
                summary = "NOTE_DELETE — удаление заметки (шифруется)",
                value = json!({
                    "type": "NOTE_DELETE",
                    "data": { "id": "note-uuid", "author": "gm", "visibility": "Public" }
                })
            )),
            // ── Scenes ────────────────────────────────────────────────────────
            ("Scene Create" = (
                summary = "SCENE_CREATE — новая сцена",
                value = json!({
                    "type": "SCENE_CREATE",
                    "data": {
                        "scene": {
                            "id": "scene-uuid",
                            "name": "Таверна Прыгающий Пони",
                            "grid": { "columns": 20, "rows": 15, "cell_size_feet": 5 },
                            "workspace_x": 0.0,
                            "workspace_y": 0.0,
                            "tokens": []
                        },
                        "actor": "gm"
                    }
                })
            )),
            ("Scene Activate" = (
                summary = "SCENE_ACTIVATE — переключить активную сцену",
                value = json!({
                    "type": "SCENE_ACTIVATE",
                    "data": { "scene_id": "scene-uuid", "actor": "gm" }
                })
            )),
            ("Token Move" = (
                summary = "TOKEN_MOVE — переместить токен",
                description = "Координаты x/y — в клетках (дробные разрешены при Ctrl+drag).",
                value = json!({
                    "type": "TOKEN_MOVE",
                    "data": {
                        "token_id": "token-uuid",
                        "scene_id": "scene-uuid",
                        "x": 5.0,
                        "y": 3.0,
                        "actor": "aragorn"
                    }
                })
            )),
            // ── Files ─────────────────────────────────────────────────────────
            ("File Announce" = (
                summary = "FILE_ANNOUNCE — объявить доступный файл (шифруется)",
                description = "Клиент объявляет, что у него есть файл. Остальные могут запросить его через FILE_REQUEST.",
                value = json!({
                    "type": "FILE_ANNOUNCE",
                    "data": {
                        "file": {
                            "hash": "a3f9b2c1d4e5f678901234567890abcd",
                            "mime_type": "image/png",
                            "file_name": "map.png",
                            "size": 204800
                        },
                        "from": "aragorn"
                    }
                })
            )),
            ("File Request" = (
                summary = "FILE_REQUEST — запросить файл у пира (шифруется)",
                value = json!({
                    "type": "FILE_REQUEST",
                    "data": { "hash": "a3f9b2c1d4e5f678901234567890abcd", "requester": "legolas" }
                })
            )),
            ("File Chunk" = (
                summary = "FILE_CHUNK — чанк файла (шифруется, лимит 1200/10 с)",
                description = "Размер чанка ~48 КБ. Индексируется с 0. Передача сборки по хешу.",
                value = json!({
                    "type": "FILE_CHUNK",
                    "data": {
                        "hash": "a3f9b2c1d4e5f678901234567890abcd",
                        "requester": "legolas",
                        "chunk_index": 0,
                        "total_chunks": 5,
                        "data": "base64-encoded-chunk-data"
                    }
                })
            )),
            // ── Voting ────────────────────────────────────────────────────────
            ("Voting Start" = (
                summary = "VOTING_START — начать голосование",
                value = json!({
                    "type": "VOTING_START",
                    "data": {
                        "voting_id": "vote-uuid",
                        "question": "Идём ли мы через Морию?",
                        "options": [
                            { "id": "yes", "text": "Да" },
                            { "id": "no", "text": "Нет" }
                        ],
                        "voting_type": "SingleChoice",
                        "actor": "gm"
                    }
                })
            )),
            ("Voting Cast" = (
                summary = "VOTING_CAST — проголосовать",
                value = json!({
                    "type": "VOTING_CAST",
                    "data": {
                        "voting_id": "vote-uuid",
                        "selected_option_ids": ["yes"],
                        "voter": "aragorn"
                    }
                })
            )),
            // ── Sync ──────────────────────────────────────────────────────────
            ("Sync Request" = (
                summary = "SYNC_REQUEST — запрос версий у пиров",
                description = "Новый клиент отправляет это сразу после подключения.",
                value = json!({ "type": "SYNC_REQUEST" })
            )),
            ("Sync Version Announce" = (
                summary = "SYNC_VERSION_ANNOUNCE — ответ с версией и хешем",
                description = "Каждый пир отвечает своей версией на SYNC_REQUEST.",
                value = json!({
                    "type": "SYNC_VERSION_ANNOUNCE",
                    "data": {
                        "username": "aragorn",
                        "version": 42,
                        "state_hash": "a1b2c3d4e5f6...",
                        "recent_hashes": ["prev-hash-1", "prev-hash-2"]
                    }
                })
            )),
            ("Sync Snapshot Request" = (
                summary = "SYNC_SNAPSHOT_REQUEST — запросить полный снапшот",
                description = "Новый клиент выбирает пира с наибольшей версией и запрашивает снапшот.",
                value = json!({
                    "type": "SYNC_SNAPSHOT_REQUEST",
                    "data": { "target_username": "aragorn" }
                })
            )),
            // ── Board tools ───────────────────────────────────────────────────
            ("Board Pointer Toggle" = (
                summary = "BOARD_POINTER — включить/выключить указатель",
                description = "Отправляется один раз при переключении режима. Трейл строится из обычных MOUSE_EVENT.",
                value = json!({
                    "type": "BOARD_POINTER",
                    "data": { "username": "aragorn", "active": true }
                })
            )),
            ("Attention Ping" = (
                summary = "ATTENTION_PING — Alt+ЛКМ, пинг внимания",
                description = "Показывает пульсирующее кольцо во всемировых координатах у всех клиентов.",
                value = json!({
                    "type": "ATTENTION_PING",
                    "data": {
                        "username": "aragorn",
                        "position": { "x": 250.0, "y": 180.0 }
                    }
                })
            )),
            // ── Crypto ────────────────────────────────────────────────────────
            ("Crypto Key Announce" = (
                summary = "CRYPTO_KEY_ANNOUNCE — публичный ключ X25519",
                description = "Отправляется при подключении и в ответ на SYNC_REQUEST от пира.",
                value = json!({
                    "type": "CRYPTO_KEY_ANNOUNCE",
                    "data": {
                        "username": "aragorn",
                        "public_key_b64": "base64-x25519-public-key-32-bytes"
                    }
                })
            )),
            ("Crypto Key Wrap" = (
                summary = "CRYPTO_KEY_WRAP — передача комнатного ключа",
                description = "Отправитель оборачивает свой комнатный ключ через ECDH для конкретного получателя.",
                value = json!({
                    "type": "CRYPTO_KEY_WRAP",
                    "data": {
                        "key_id": "3e462f13-3941-4e7b-b8c6-b0c684f2f8f2",
                        "sender_username": "aragorn",
                        "recipient_username": "legolas",
                        "sender_public_key_b64": "base64-sender-pubkey",
                        "nonce_b64": "base64-12-byte-nonce",
                        "wrapped_key_b64": "base64-chacha20-ciphertext"
                    }
                })
            )),
            ("Crypto Payload Chat" = (
                summary = "CRYPTO_PAYLOAD — зашифрованное событие",
                description = "Оборачивает CHAT_MESSAGE, DIRECT_MESSAGE, NOTE_*, SYNC_SNAPSHOT, FILE_*. Поле kind указывает тип.",
                value = json!({
                    "type": "CRYPTO_PAYLOAD",
                    "data": {
                        "version": 1,
                        "key_id": "3e462f13-3941-4e7b-b8c6-b0c684f2f8f2",
                        "sender_username": "aragorn",
                        "kind": "CHAT",
                        "nonce_b64": "base64-12-byte-nonce",
                        "ciphertext_b64": "base64-chacha20-poly1305-ciphertext"
                    }
                })
            )),
            // ── Misc ──────────────────────────────────────────────────────────
            ("Ping" = (
                summary = "PING — keepalive",
                description = "Сервер отвечает {\"type\":\"PONG\"}.",
                value = json!({ "type": "PING" })
            ))
        )
    ),
    responses(
        (status = 101, description = "Switching Protocols — соединение установлено"),
        (status = 200,
            description = "Входящие сообщения от сервера (ретрансляция событий других клиентов). \
                Формат идентичен исходящим: те же типы ClientEvent. \
                Зашифрованный контент приходит как CRYPTO_PAYLOAD.",
            body = ClientEvent)
    )
)]
pub async fn websocket_docs() {}
