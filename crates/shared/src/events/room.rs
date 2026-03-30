use crate::events::chat::ChatMessagePayload;
use crate::events::scene::Scene;
use crate::events::voting::VotingResultPayload;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
#[cfg(feature = "schemas")]
use utoipa::ToSchema;
#[cfg(feature = "validation")]
use validator::Validate;

const MAX_HISTORY_LOG_ENTRIES: usize = 500;

/// Полное состояние комнаты, которое мы синхронизируем
#[derive(Debug, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "validation", derive(Validate))]
#[cfg_attr(feature = "schemas", derive(ToSchema))]
pub struct RoomState {
    /// История сообщений чата
    pub chat_history: Vec<ChatMessagePayload>,

    /// Результаты завершенных голосований (voting_id -> результаты)
    #[serde(default)]
    pub voting_results: HashMap<String, VotingResultPayload>,

    /// Список сцен комнаты
    #[serde(default)]
    pub scenes: Vec<Scene>,

    /// Активная сцена, синхронизируемая между участниками
    #[serde(default)]
    pub active_scene_id: Option<String>,

    /// Версия состояния
    pub version: u64,

    /// Текущий хеш состояния
    pub current_hash: String,

    /// История версий (последние MAX_HISTORY_LOG_ENTRIES): (версия, хеш)
    #[serde(default)]
    pub history_log: Vec<(u64, String)>,
}

impl Default for RoomState {
    fn default() -> Self {
        let mut state = Self {
            chat_history: Vec::new(),
            voting_results: HashMap::new(),
            scenes: Vec::new(),
            active_scene_id: None,
            version: 0,
            current_hash: String::new(),
            history_log: Vec::new(),
        };
        // Вычисляем начальный хеш для пустого состояния
        state.compute_hash();
        state
    }
}

impl RoomState {
    /// Вычисляет хеш текущего состояния
    fn compute_hash(&mut self) -> String {
        let mut hasher = Sha256::new();

        // Хешируем полезную нагрузку
        if let Ok(chat_json) = serde_json::to_string(&self.chat_history) {
            hasher.update(chat_json.as_bytes());
        }

        // Хешируем результаты голосований
        if let Ok(voting_json) = serde_json::to_string(&self.voting_results) {
            hasher.update(voting_json.as_bytes());
        }

        if let Ok(scenes_json) = serde_json::to_string(&self.scenes) {
            hasher.update(scenes_json.as_bytes());
        }

        if let Ok(active_scene_json) = serde_json::to_string(&self.active_scene_id) {
            hasher.update(active_scene_json.as_bytes());
        }

        // Добавляем ссылку на предыдущий хеш (как цепочку блоков)
        hasher.update(self.current_hash.as_bytes());

        let result = hasher.finalize();
        format!("{:x}", result)
    }

    /// Фиксирует изменения: инкрементирует версию, обновляет хеш и историю
    pub fn commit_changes(&mut self) {
        self.version += 1;
        let new_hash = self.compute_hash();

        // Добавляем текущую версию-хеш в историю
        self.history_log.push((self.version, new_hash.clone()));

        if self.history_log.len() > MAX_HISTORY_LOG_ENTRIES {
            self.history_log
                .drain(0..self.history_log.len() - MAX_HISTORY_LOG_ENTRIES);
        }

        self.current_hash = new_hash;
    }

    /// Проверяет, есть ли версия с данным хешом в истории
    pub fn has_version_with_hash(&self, version: u64, hash: &str) -> bool {
        self.history_log
            .iter()
            .any(|(v, h)| *v == version && h == hash)
    }
}
