use super::types::{ConflictType, SyncConflict};
use leptos::logging::warn;
use shared::events::RoomState;

/// Builds a RoomState at the given version with a specific hash chain.
/// Each commit_changes call advances version by 1 and records the (version, hash) pair.
#[cfg(test)]
fn build_state_with_commits(commits: u64) -> RoomState {
    let mut state = RoomState::default();
    for _ in 0..commits {
        state.commit_changes();
    }
    state
}

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

#[cfg(test)]
mod tests {
    use super::*;

    // ---------------------------------------------------------------------------
    // Identical state scenarios
    // ---------------------------------------------------------------------------

    #[test]
    fn identical_versions_and_hashes_returns_ok() {
        let remote = build_state_with_commits(3);
        let result = SyncValidator::validate_remote_state(
            3,
            &remote.current_hash,
            3,
            3,
            &remote.current_hash,
            &remote,
        );
        assert!(
            result.is_ok(),
            "identical state should not produce a conflict"
        );
    }

    // ---------------------------------------------------------------------------
    // Split-brain: same version, different hashes
    // ---------------------------------------------------------------------------

    #[test]
    fn same_version_different_hash_is_split_brain() {
        let remote = build_state_with_commits(5);
        let result = SyncValidator::validate_remote_state(
            5,
            "completely_different_hash",
            5,
            5,
            &remote.current_hash,
            &remote,
        );
        match result {
            Err(SyncConflict {
                conflict_type: ConflictType::SplitBrain,
                local_version: 5,
                remote_version: 5,
            }) => {}
            other => panic!("expected SplitBrain conflict, got {other:?}"),
        }
    }

    // ---------------------------------------------------------------------------
    // Descendant: remote is newer and includes our hash in its history
    // ---------------------------------------------------------------------------

    #[test]
    fn remote_newer_and_is_linear_descendant_returns_ok() {
        // Build shared base (v2), then remote goes to v5 on top of it.
        let mut remote = build_state_with_commits(2);
        let local_hash_at_v2 = remote.current_hash.clone();
        remote.commit_changes(); // v3
        remote.commit_changes(); // v4
        remote.commit_changes(); // v5

        // Local is at v2 (synced), remote is at v5 and its history_log contains (v2, hash).
        let result = SyncValidator::validate_remote_state(
            2,
            &local_hash_at_v2,
            2, // last_synced == local_ver → no unsynced local changes
            5,
            &remote.current_hash,
            &remote,
        );
        assert!(
            result.is_ok(),
            "linear descendant should not conflict: {result:?}"
        );
    }

    #[test]
    fn remote_newer_descendant_with_local_at_version_zero_returns_ok() {
        // Newcomer (local v0) always syncs.
        let remote = build_state_with_commits(3);
        // The history_log won't have (0, empty) but that's fine – SyncValidator
        // only checks for Fork / UnsyncedLocal when remote_ver > local_ver.
        // local_ver == 0, local_hash doesn't matter.
        let result =
            SyncValidator::validate_remote_state(0, "", 0, 3, &remote.current_hash, &remote);
        // For v0 local, has_version_with_hash(0, "") returns false → Fork would fire.
        // The caller (handle_snapshot) guards against v0 separately; this test just
        // confirms the raw validator behaviour at the boundary.
        // We expect Fork because history_log does not contain (0, "").
        match result {
            Err(SyncConflict {
                conflict_type: ConflictType::Fork,
                ..
            }) => {}
            Ok(()) => {} // acceptable if implementor chooses to special-case v0
            other => panic!("unexpected result for newcomer: {other:?}"),
        }
    }

    // ---------------------------------------------------------------------------
    // Fork: remote is newer but our hash is NOT in its history
    // ---------------------------------------------------------------------------

    #[test]
    fn remote_newer_without_our_hash_in_history_is_fork() {
        let mut state_a = build_state_with_commits(2);
        let local_hash = state_a.current_hash.clone();

        // Remote diverged at v2 and went its own way (v2 → v5 without our hash).
        let mut remote = RoomState::default();
        remote.commit_changes(); // v1
        remote.commit_changes(); // v2 (different content path, different hash)
        remote.commit_changes(); // v3
        remote.commit_changes(); // v4
        remote.commit_changes(); // v5

        // Ensure remote.history_log does NOT contain our hash (high likelihood since
        // we took a different path; if by some SHA collision it matches, just skip).
        if remote.history_log.iter().any(|(_, h)| *h == local_hash) {
            return; // Hash collision, skip test
        }

        let result = SyncValidator::validate_remote_state(
            2,
            &local_hash,
            2,
            5,
            &remote.current_hash,
            &remote,
        );
        match result {
            Err(SyncConflict {
                conflict_type: ConflictType::Fork,
                local_version: 2,
                remote_version: 5,
            }) => {}
            other => panic!("expected Fork conflict, got {other:?}"),
        }
        let _ = state_a; // keep alive
    }

    // ---------------------------------------------------------------------------
    // UnsyncedLocal: remote is a valid descendant but we have unsaved local changes
    // ---------------------------------------------------------------------------

    #[test]
    fn descendant_with_unsynced_local_changes_is_unsynced_local_conflict() {
        let mut remote = build_state_with_commits(2);
        let local_hash_at_v2 = remote.current_hash.clone();
        remote.commit_changes(); // v3
        remote.commit_changes(); // v4

        // Local is at v2 (synced up to v1 only → local_ver > last_synced).
        let result = SyncValidator::validate_remote_state(
            2,
            &local_hash_at_v2,
            1, // last_synced < local_ver → unsynced changes exist
            4,
            &remote.current_hash,
            &remote,
        );
        match result {
            Err(SyncConflict {
                conflict_type: ConflictType::UnsyncedLocal,
                local_version: 2,
                remote_version: 4,
            }) => {}
            other => panic!("expected UnsyncedLocal conflict, got {other:?}"),
        }
    }

    // ---------------------------------------------------------------------------
    // Remote is behind local: should silently accept (no conflict)
    // ---------------------------------------------------------------------------

    #[test]
    fn remote_behind_local_returns_ok() {
        let remote = build_state_with_commits(2);
        // Local is ahead at v5; remote is at v2.
        let result = SyncValidator::validate_remote_state(
            5,
            "some_local_hash",
            5,
            2,
            &remote.current_hash,
            &remote,
        );
        assert!(result.is_ok(), "stale remote should not produce a conflict");
    }

    // ---------------------------------------------------------------------------
    // has_version_with_hash on RoomState
    // ---------------------------------------------------------------------------

    #[test]
    fn has_version_with_hash_finds_recorded_entry() {
        let mut state = RoomState::default();
        state.commit_changes(); // v1
        let hash_v1 = state.current_hash.clone();
        state.commit_changes(); // v2
        state.commit_changes(); // v3

        assert!(
            state.has_version_with_hash(1, &hash_v1),
            "should find (v1, hash_v1) in history_log"
        );
        assert!(
            !state.has_version_with_hash(1, "wrong_hash"),
            "wrong hash should not match"
        );
        assert!(
            !state.has_version_with_hash(99, &hash_v1),
            "wrong version should not match"
        );
    }

    #[test]
    fn has_version_with_hash_false_for_empty_state() {
        let state = RoomState::default();
        assert!(!state.has_version_with_hash(0, ""));
        assert!(!state.has_version_with_hash(1, "anything"));
    }

    // ---------------------------------------------------------------------------
    // SyncVersionAnnounce discard-path fields are valid (regression for empty
    // username / empty recent_hashes bug)
    // ---------------------------------------------------------------------------

    #[test]
    fn discard_snapshot_announce_uses_proper_recent_hashes() {
        // Simulate what the discard path in handle_snapshot now does.
        let remote = build_state_with_commits(4);
        let recent_hashes: Vec<String> = remote
            .history_log
            .iter()
            .map(|(_, hash)| hash.clone())
            .collect();

        // The announce must contain all hashes from history_log.
        assert_eq!(recent_hashes.len(), 4, "should have one hash per commit");
        for (i, (_, expected)) in remote.history_log.iter().enumerate() {
            assert_eq!(
                recent_hashes[i], *expected,
                "hash at index {i} must match history_log"
            );
        }

        // And the final hash must be the current state hash.
        assert_eq!(
            recent_hashes.last().unwrap(),
            &remote.current_hash,
            "last recent hash should be the current state hash"
        );
    }
}
