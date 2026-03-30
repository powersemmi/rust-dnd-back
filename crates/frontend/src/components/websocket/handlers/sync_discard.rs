use crate::components::websocket::WsSender;
use gloo_timers::future::TimeoutFuture;
use leptos::logging::log;
use leptos::task::spawn_local;
use shared::events::{
    ClientEvent, SyncSnapshotRequestPayload, SyncVersionPayload,
    voting::{VotingOption, VotingStartPayload, VotingType},
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub struct ConflictResolutionContext<'a> {
    pub tx: &'a WsSender,
    pub collected_announces: &'a Rc<RefCell<Vec<SyncVersionPayload>>>,
    pub is_collecting_announces: &'a Rc<RefCell<bool>>,
    pub expected_snapshot_from: &'a Rc<RefCell<Option<String>>>,
}

/// Запуск процесса разрешения конфликта через сбор анонсов
pub fn start_conflict_resolution(ctx: ConflictResolutionContext<'_>) {
    log!("🔄 Starting conflict resolution via announce collection...");

    let tx_clone = ctx.tx.clone();
    let collected_announces_clone = ctx.collected_announces.clone();
    let is_collecting_announces_clone = ctx.is_collecting_announces.clone();
    let expected_snapshot_from_clone = ctx.expected_snapshot_from.clone();

    spawn_local(async move {
        // Включаем режим сбора анонсов
        *is_collecting_announces_clone.borrow_mut() = true;
        collected_announces_clone.borrow_mut().clear();

        log!("📢 Announce collection mode enabled");

        // Отправляем SyncRequest - все отправят свои анонсы
        let sync_request = ClientEvent::SyncRequest;
        if tx_clone.try_send_event(sync_request).is_ok() {
            log!("📤 Sent SyncRequest broadcast");
        }

        // Ждём 2 секунды для получения всех анонсов
        TimeoutFuture::new(2000).await;

        log!("⏱️ Collection timeout reached, analyzing announces...");

        // Выключаем режим сбора
        *is_collecting_announces_clone.borrow_mut() = false;

        // Анализируем собранные анонсы
        analyze_announces_and_resolve(
            &collected_announces_clone,
            &tx_clone,
            &expected_snapshot_from_clone,
        );
    });
}

/// Обработка входящего анонса во время сбора для конфликт-резолюции
pub fn handle_announce_for_conflict(
    payload: SyncVersionPayload,
    collected_announces: &Rc<RefCell<Vec<SyncVersionPayload>>>,
) {
    let hash_preview = if payload.state_hash.is_empty() {
        "<empty>"
    } else {
        &payload.state_hash[..8.min(payload.state_hash.len())]
    };
    log!(
        "📥 Collecting announce for conflict resolution: {} v{} (hash: {}...)",
        payload.username,
        payload.version,
        hash_preview
    );

    // Дедуплицируем: если уже есть анонс от этого пользователя, заменяем его
    let mut announces = collected_announces.borrow_mut();
    if let Some(existing) = announces
        .iter_mut()
        .find(|a| a.username == payload.username)
    {
        log!("🔄 Updating existing announce from {}", payload.username);
        *existing = payload;
    } else {
        announces.push(payload);
    }

    log!(
        "📊 Total collected announces: {} (unique users)",
        announces.len()
    );
}

/// Анализ собранных анонсов и принятие решения
fn analyze_announces_and_resolve(
    collected_announces: &Rc<RefCell<Vec<SyncVersionPayload>>>,
    tx: &WsSender,
    expected_snapshot_from: &Rc<RefCell<Option<String>>>,
) {
    let announces = collected_announces.borrow().clone();
    log!(
        "📊 [DISCARD] Analyzing {} collected announces",
        announces.len()
    );

    // Логируем каждый анонс для отладки
    for (i, announce) in announces.iter().enumerate() {
        let hash_preview = if announce.state_hash.is_empty() {
            "<empty>"
        } else {
            &announce.state_hash[..8.min(announce.state_hash.len())]
        };
        log!(
            "  [DISCARD] Announce #{}: user={}, v={}, hash={}...",
            i + 1,
            announce.username,
            announce.version,
            hash_preview
        );
    }

    if announces.is_empty() {
        log!("⚠️ No announces collected!");
        return;
    }

    // Подсчитываем количество одинаковых хешей
    let mut hash_counts: HashMap<String, Vec<SyncVersionPayload>> = HashMap::new();
    for announce in announces {
        hash_counts
            .entry(announce.state_hash.clone())
            .or_default()
            .push(announce);
    }

    log!("🔍 Found {} unique hashes:", hash_counts.len());
    for (hash, announces_list) in &hash_counts {
        let hash_preview = if hash.is_empty() {
            "<empty>"
        } else {
            &hash[..8.min(hash.len())]
        };
        log!(
            "  - {}... : {} users (v{})",
            hash_preview,
            announces_list.len(),
            announces_list[0].version
        );
    }

    let total_announces = collected_announces.borrow().len();

    // Случай 1: Если собран только 1 анонс - это значит 2 участника (инициатор уже очистил стейт)
    // Автоматически применяем этот единственный вариант
    if total_announces == 1 {
        log!(
            "✅ [DISCARD CASE 1] Only 1 announce collected (2 participants total) - auto-applying"
        );
        let announce = &collected_announces.borrow()[0];
        request_snapshot_from_user(&announce.username, tx, expected_snapshot_from);
        return;
    }

    // Случай 1.5: Если собрано ровно 2 анонса - это 2 участника в split-brain
    // Автоматически выбираем детерминированно (лексикографически первый username)
    // чтобы оба пользователя сделали одинаковый выбор
    if total_announces == 2 {
        let mut announces_sorted = collected_announces.borrow().clone();
        announces_sorted.sort_by(|a, b| a.username.cmp(&b.username));
        let selected_user = &announces_sorted[0].username;

        log!(
            "✅ [DISCARD CASE 1.5] Exactly 2 announces (2 participants in split-brain) - auto-selecting: {}",
            selected_user
        );
        request_snapshot_from_user(selected_user, tx, expected_snapshot_from);
        return;
    }

    // Случай 2: Если все анонсы с одним хешем - единогласие
    if hash_counts.len() == 1 {
        log!("✅ [DISCARD CASE 2] All announces have same hash - unanimous agreement");
        let (_, announces_list) = hash_counts.iter().next().unwrap();
        request_snapshot_from_user(&announces_list[0].username, tx, expected_snapshot_from);
        return;
    }

    // Случай 3: Ищем большинство (>50%)
    let majority_threshold = total_announces / 2;
    log!(
        "🔍 [DISCARD] Checking for majority: need > {} out of {}",
        majority_threshold,
        total_announces
    );

    if let Some((_majority_hash, majority_announces)) = hash_counts
        .iter()
        .find(|(_, announces_list)| announces_list.len() > majority_threshold)
    {
        // Есть явное большинство - запрашиваем snapshot от одного пользователя
        log!(
            "✅ [DISCARD CASE 3] Found majority with {} votes (>50%)",
            majority_announces.len()
        );
        request_snapshot_from_user(&majority_announces[0].username, tx, expected_snapshot_from);
    } else {
        // Случай 4: 3+ участника БЕЗ большинства - создаём голосование
        log!(
            "⚠️ [DISCARD CASE 4] No clear majority with {} participants, creating voting...",
            total_announces
        );
        create_hash_selection_voting(hash_counts, tx);
    }
}

/// Запрос полного snapshot от выбранного пользователя
fn request_snapshot_from_user(
    target_username: &str,
    tx: &WsSender,
    expected_snapshot_from: &Rc<RefCell<Option<String>>>,
) {
    log!("📥 Requesting full snapshot from user: {}", target_username);

    // Устанавливаем ожидание snapshot для отключения валидации
    *expected_snapshot_from.borrow_mut() = Some(target_username.to_string());
    log!("🔓 Set expected_snapshot_from to: {}", target_username);

    let request = ClientEvent::SyncSnapshotRequest(SyncSnapshotRequestPayload {
        target_username: target_username.to_string(),
    });

    if tx.try_send_event(request).is_ok() {
        log!("📤 Sent SyncSnapshotRequest to {}", target_username);
    }
}

/// Создание голосования для выбора версии при отсутствии большинства
fn create_hash_selection_voting(
    hash_counts: HashMap<String, Vec<SyncVersionPayload>>,
    tx: &WsSender,
) {
    log!("⚠️ No clear majority, creating voting for hash selection...");

    // Создаём варианты для голосования
    let mut voting_options: Vec<VotingOption> = hash_counts
        .iter()
        .enumerate()
        .map(|(idx, (hash, announces_list))| {
            let count = announces_list.len();
            let version = announces_list[0].version;
            let hash_short = &hash[hash.len().saturating_sub(6)..];

            VotingOption {
                id: format!("hash_{}", idx),
                text: format!("{} members - {} v{}", count, hash_short, version),
            }
        })
        .collect();

    // Сортируем по количеству участников (больше голосов сверху)
    voting_options.sort_by(|a, b| {
        let a_count: u32 = a
            .text
            .split_whitespace()
            .next()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let b_count: u32 = b
            .text
            .split_whitespace()
            .next()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        b_count.cmp(&a_count)
    });

    let voting_id = format!("hash_select_{}", js_sys::Date::now() as u64);

    let voting_payload = VotingStartPayload {
        voting_id,
        question: "conflict.select_version".to_string(), // i18n key
        options: voting_options,
        voting_type: VotingType::SingleChoice,
        is_anonymous: false,
        timer_seconds: Some(60),
        default_option_id: None,
        creator: "system".to_string(),
    };

    let event = ClientEvent::VotingStart(voting_payload);
    if tx.try_send_event(event).is_ok() {
        log!("🗳️ Created hash selection voting");
    }
}
