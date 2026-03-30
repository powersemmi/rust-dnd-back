use crate::components::statistics::StateEvent;
use crate::components::websocket::{WsSender, storage, sync::SyncValidator, types::*, utils};
use leptos::logging::log;
use leptos::prelude::*;
use shared::events::{
    ClientEvent, RoomState, SyncSnapshotPayload, SyncSnapshotRequestPayload, SyncVersionPayload,
};
use std::cell::RefCell;
use std::rc::Rc;

use super::HandlerContext;

pub fn handle_sync_request(
    tx: &WsSender,
    room_state: &Rc<RefCell<RoomState>>,
    local_version: &Rc<RefCell<u64>>,
    my_username: &str,
) {
    let current_ver = *local_version.borrow();
    let state = room_state.borrow();
    let state_hash = state.current_hash.clone();

    // Извлекаем последние 500 хешей из истории
    let recent_hashes: Vec<String> = state
        .history_log
        .iter()
        .map(|(_, hash)| hash.clone())
        .collect();

    let announce = ClientEvent::SyncVersionAnnounce(SyncVersionPayload {
        username: my_username.to_string(),
        version: current_ver,
        state_hash,
        recent_hashes,
    });
    let _ = tx.try_send_event(announce);
}

pub fn handle_sync_announce(payload: SyncVersionPayload, ctx: &HandlerContext<'_>) {
    // Если мы в режиме сбора анонсов для конфликт-резолюции, собираем и возвращаем
    if *ctx.is_collecting_announces.borrow() {
        use super::sync_discard;
        sync_discard::handle_announce_for_conflict(payload, ctx.collected_announces);
        return;
    }

    let my_ver = *ctx.local_version.borrow();
    let state = ctx.room_state.borrow();
    let my_hash = state.current_hash.clone();

    // Специальная обработка для новичков (версия 0 или пустой хеш)
    let i_am_newcomer = my_ver == 0 || my_hash.is_empty();
    let they_are_newcomer = payload.version == 0 || payload.state_hash.is_empty();

    // Если они новичок - просто игнорируем их анонс, не создаём конфликтов
    if they_are_newcomer {
        log!(
            "🆕 {} is a newcomer (v{}, empty hash), ignoring",
            payload.username,
            payload.version
        );
        return;
    }

    // Если я новичок и вижу кого-то с непустым состоянием
    if i_am_newcomer && !they_are_newcomer {
        log!(
            "🆕 I'm a newcomer, {} has state v{} (hash: {}...)",
            payload.username,
            payload.version,
            &payload.state_hash[..8.min(payload.state_hash.len())]
        );

        // Добавляем в кандидаты для синхронизации
        ctx.sync_candidates
            .borrow_mut()
            .push((payload.username.clone(), payload.version));

        utils::log_event(
            ctx.state_events,
            my_ver,
            "SYNC_VERSION_ANNOUNCE",
            &format!(
                "{} announced v{} (newcomer will sync)",
                payload.username, payload.version
            ),
        );
        return;
    }

    // Далее - оба НЕ новички, проверяем линию развития состояния (lineage check)
    let lineage_status = if my_hash == payload.state_hash {
        // Одинаковые хеши - идентичные состояния
        log!("Identical states with {}: same hash", payload.username);

        // Если у нас был конфликт, но теперь состояния идентичны - очищаем конфликт
        if ctx.conflict_signal.get().is_some() {
            log!("✅ Conflict resolved - states are now identical");
            ctx.conflict_signal.set(None);
        }

        "IDENTICAL"
    } else if payload.version > my_ver {
        // Удалённая версия новее - проверяем, является ли она потомком нашего состояния
        let remote_has_our_state = payload.recent_hashes.iter().any(|h| h == &my_hash);

        if remote_has_our_state {
            log!(
                "{} is ahead (v{}) and is descendant of our state (v{}) - safe to sync",
                payload.username,
                payload.version,
                my_ver
            );
            "DESCENDANT"
        } else {
            // Удалённая версия новее, но не содержит нашу версию - это форк
            log::warn!(
                "FORK detected with {}: they are at v{}, we are at v{}, but no common lineage",
                payload.username,
                payload.version,
                my_ver
            );

            // Устанавливаем конфликт ТОЛЬКО если не в режиме сбора анонсов
            if !*ctx.is_collecting_announces.borrow() {
                ctx.conflict_signal.set(Some(SyncConflict {
                    conflict_type: ConflictType::Fork,
                    local_version: my_ver,
                    remote_version: payload.version,
                }));
            } else {
                log!("⚠️ Fork detected but ignoring (in announce collection mode)");
            }

            "FORK"
        }
    } else if payload.version < my_ver {
        // Удалённая версия старше - они отстают
        log!(
            "{} is behind: v{} < our v{}",
            payload.username,
            payload.version,
            my_ver
        );
        "BEHIND"
    } else {
        // Одинаковые версии, но разные хеши - split brain
        log::warn!(
            "SPLIT BRAIN with {}: same version v{}, different hashes",
            payload.username,
            my_ver
        );

        // Устанавливаем конфликт ТОЛЬКО если не в режиме сбора анонсов
        // (иначе получается бесконечный цикл открытия окон конфликта)
        if !*ctx.is_collecting_announces.borrow() {
            ctx.conflict_signal.set(Some(SyncConflict {
                conflict_type: ConflictType::SplitBrain,
                local_version: my_ver,
                remote_version: payload.version,
            }));
        } else {
            log!("⚠️ Split brain detected but ignoring (in announce collection mode)");
        }

        "SPLIT_BRAIN"
    };

    drop(state);

    // Добавляем в кандидаты только если это не форк и не split brain
    if lineage_status != "FORK" && lineage_status != "SPLIT_BRAIN" {
        ctx.sync_candidates
            .borrow_mut()
            .push((payload.username.clone(), payload.version));
    }

    let hash_preview = if payload.state_hash.is_empty() {
        "<empty>"
    } else {
        &payload.state_hash[..8.min(payload.state_hash.len())]
    };
    utils::log_event(
        ctx.state_events,
        my_ver,
        "SYNC_VERSION_ANNOUNCE",
        &format!(
            "{} announced v{} (status: {}, hash: {}...)",
            payload.username, payload.version, lineage_status, hash_preview
        ),
    );
}

pub fn handle_snapshot_request(
    payload: SyncSnapshotRequestPayload,
    tx: &WsSender,
    room_state: &Rc<RefCell<RoomState>>,
    local_version: &Rc<RefCell<u64>>,
    my_username: &str,
    state_events: RwSignal<Vec<StateEvent>>,
) {
    // Отвечаем если это адресовано нам или broadcast (пустая строка)
    if payload.target_username == my_username || payload.target_username.is_empty() {
        log!(
            "Sending snapshot to requester (broadcast: {})",
            payload.target_username.is_empty()
        );
        let state = room_state.borrow().clone();
        let snapshot = ClientEvent::SyncSnapshot(SyncSnapshotPayload {
            version: state.version,
            state: state.clone(),
        });
        let _ = tx.try_send_event(snapshot);

        utils::log_event(
            state_events,
            *local_version.borrow(),
            "SYNC_SNAPSHOT_SENT",
            &format!("Sent snapshot v{} to requester", state.version),
        );
    }
}

pub fn handle_snapshot(payload: SyncSnapshotPayload, ctx: &HandlerContext<'_>) {
    // Проверяем, находимся ли мы в режиме сбора snapshots
    if *ctx.is_collecting_snapshots.borrow() {
        log!(
            "📦 Collecting snapshot for analysis (v{}, hash: {}...)",
            payload.state.version,
            &payload.state.current_hash[..8]
        );

        // Добавляем snapshot в коллекцию
        ctx.collected_snapshots
            .borrow_mut()
            .push((ctx.my_username.to_string(), payload.state.clone()));

        log!(
            "📊 Total collected snapshots: {}",
            ctx.collected_snapshots.borrow().len()
        );
        return; // Не применяем snapshot, только собираем
    }

    let local_state = ctx.room_state.borrow();
    let local_ver = local_state.version;
    let local_hash = local_state.current_hash.clone();
    let local_synced = *ctx.last_synced_version.borrow();
    drop(local_state);

    let remote_ver = payload.state.version;
    let remote_hash = &payload.state.current_hash;

    log!(
        "Validating snapshot: local v{} (hash: {}), remote v{} (hash: {})",
        local_ver,
        local_hash,
        remote_ver,
        remote_hash
    );

    // Проверяем, ожидаем ли мы снапшот после конфликт-резолвинга
    let is_conflict_resolution_snapshot = ctx.expected_snapshot_from.borrow().is_some();

    if is_conflict_resolution_snapshot {
        log!("🔄 This is a conflict resolution snapshot, applying without strict validation");

        // Применяем снапшот
        *ctx.local_version.borrow_mut() = remote_ver;
        *ctx.last_synced_version.borrow_mut() = remote_ver;
        *ctx.room_state.borrow_mut() = payload.state.clone();

        ctx.messages_signal.set(payload.state.chat_history.clone());
        ctx.public_notes_signal
            .set(payload.state.public_notes.clone());
        ctx.file_transfer
            .reconcile_chat_attachments(&payload.state.chat_history);
        ctx.voting_results.set(payload.state.voting_results.clone());
        ctx.scenes_signal.set(payload.state.scenes.clone());
        ctx.active_scene_id_signal
            .set(payload.state.active_scene_id.clone());
        storage::save_state_in_background(ctx.room_name, &payload.state);

        // Очищаем ожидание и закрываем окно конфликта
        *ctx.expected_snapshot_from.borrow_mut() = None;
        ctx.conflict_signal.set(None);

        log!("✅ Conflict resolution snapshot applied: v{}", remote_ver);

        utils::log_event(
            ctx.state_events,
            remote_ver,
            "CONFLICT_RESOLVED",
            &format!("Applied conflict resolution snapshot v{}", remote_ver),
        );
        return;
    }

    // Если локальная версия 0 (начальное состояние после discard), принимаем любой snapshot
    if local_ver == 0 && remote_ver > 0 {
        log!(
            "📥 Accepting snapshot after discard (local v0 -> remote v{})",
            remote_ver
        );

        *ctx.local_version.borrow_mut() = remote_ver;
        *ctx.last_synced_version.borrow_mut() = remote_ver;
        *ctx.room_state.borrow_mut() = payload.state.clone();

        ctx.messages_signal.set(payload.state.chat_history.clone());
        ctx.public_notes_signal
            .set(payload.state.public_notes.clone());
        ctx.file_transfer
            .reconcile_chat_attachments(&payload.state.chat_history);
        ctx.voting_results.set(payload.state.voting_results.clone());
        ctx.scenes_signal.set(payload.state.scenes.clone());
        ctx.active_scene_id_signal
            .set(payload.state.active_scene_id.clone());
        storage::save_state_in_background(ctx.room_name, &payload.state);

        ctx.conflict_signal.set(None);

        utils::log_event(
            ctx.state_events,
            remote_ver,
            "DISCARD_SNAPSHOT_RECEIVED",
            &format!("Applied snapshot v{} after discard", remote_ver),
        );

        // Отправляем SyncVersionAnnounce чтобы другие пользователи знали что конфликт разрешен
        let announce = ClientEvent::SyncVersionAnnounce(shared::events::SyncVersionPayload {
            username: String::new(), // Будет заполнено сервером
            version: remote_ver,
            state_hash: payload.state.current_hash.clone(),
            recent_hashes: vec![],
        });
        if ctx.tx.try_send_event(announce).is_ok() {
            log!("📢 Sent SyncVersionAnnounce after discard");
        }

        return;
    }

    // Обычная валидация через SyncValidator
    match SyncValidator::validate_remote_state(
        local_ver,
        &local_hash,
        local_synced,
        remote_ver,
        remote_hash,
        &payload.state,
    ) {
        Err(conflict) => {
            ctx.conflict_signal.set(Some(conflict.clone()));
            utils::log_event(
                ctx.state_events,
                local_ver,
                "SYNC_CONFLICT",
                &format!(
                    "Conflict detected: local v{} vs remote v{}",
                    local_ver, remote_ver
                ),
            );
        }
        Ok(()) if remote_ver > local_ver => {
            // Валидация пройдена, применяем стейт
            log!("Applying snapshot v{}", remote_ver);

            *ctx.local_version.borrow_mut() = remote_ver;
            *ctx.last_synced_version.borrow_mut() = remote_ver;
            *ctx.room_state.borrow_mut() = payload.state.clone();

            ctx.messages_signal.set(payload.state.chat_history.clone());
            ctx.public_notes_signal
                .set(payload.state.public_notes.clone());
            ctx.file_transfer
                .reconcile_chat_attachments(&payload.state.chat_history);
            ctx.voting_results.set(payload.state.voting_results.clone());
            ctx.scenes_signal.set(payload.state.scenes.clone());
            ctx.active_scene_id_signal
                .set(payload.state.active_scene_id.clone());
            storage::save_state_in_background(ctx.room_name, &payload.state);

            // Очищаем конфликт при успешной синхронизации
            ctx.conflict_signal.set(None);

            utils::log_event(
                ctx.state_events,
                remote_ver,
                "SYNC_SNAPSHOT_RECEIVED",
                &format!(
                    "Applied snapshot v{} ({} messages)",
                    remote_ver,
                    payload.state.chat_history.len()
                ),
            );
        }
        _ => {}
    }
}
