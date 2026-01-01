use leptos::prelude::*;

#[derive(Clone, Debug)]
pub enum ConflictType {
    SplitBrain,    // Версии равны, хеши различны
    Fork,          // Наш хеш не найден в истории удалённого стейта
    UnsyncedLocal, // Удалённый стейт новее, но у нас есть несинхронизированные изменения
}

#[derive(Clone, Debug)]
pub struct SyncConflict {
    pub conflict_type: ConflictType,
    pub local_version: u64,
    pub remote_version: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CursorSignals {
    pub x: ReadSignal<i32>,
    pub set_x: WriteSignal<i32>,
    pub y: ReadSignal<i32>,
    pub set_y: WriteSignal<i32>,
}
