use super::types::{ConflictType, SyncConflict};
use leptos::logging::warn;
use shared::events::RoomState;

pub struct SyncValidator;

impl SyncValidator {
    /// Валидирует удалённое состояние относительно локального
    pub fn validate_remote_state(
        local_ver: u64,
        local_hash: &str,
        local_synced: u64,
        remote_ver: u64,
        remote_hash: &str,
        remote_state: &RoomState,
    ) -> Result<(), SyncConflict> {
        // Сценарий 1: Версии равны
        if remote_ver == local_ver {
            if remote_hash != local_hash {
                warn!("CONFLICT: Split Brain detected! Same version but different hashes.");
                return Err(SyncConflict {
                    conflict_type: ConflictType::SplitBrain,
                    local_version: local_ver,
                    remote_version: remote_ver,
                });
            }
            return Ok(());
        }

        // Сценарий 2: Удалённая версия новее
        if remote_ver > local_ver {
            // Проверка Fork: наш хеш должен быть в истории удалённого стейта
            if !remote_state.has_version_with_hash(local_ver, local_hash) {
                warn!("CONFLICT: Fork detected! Our hash not found in remote history.");
                return Err(SyncConflict {
                    conflict_type: ConflictType::Fork,
                    local_version: local_ver,
                    remote_version: remote_ver,
                });
            }

            // Проверка несинхронизированных локальных изменений
            if local_ver > local_synced {
                warn!("CONFLICT: Unsynced local changes detected!");
                return Err(SyncConflict {
                    conflict_type: ConflictType::UnsyncedLocal,
                    local_version: local_ver,
                    remote_version: remote_ver,
                });
            }
        }

        Ok(())
    }
}
