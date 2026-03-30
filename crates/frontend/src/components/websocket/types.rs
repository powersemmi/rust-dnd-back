use leptos::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

type ConflictResolutionCallback = Rc<dyn Fn()>;
type ConflictResolutionCallbackSlot = Rc<RefCell<Option<ConflictResolutionCallback>>>;

// Обертка для callback-а, которую можно клонировать и передавать как Send/Sync
#[derive(Clone, Default)]
pub struct ConflictResolutionHandle {
    // Внутри храним Option<Rc<dyn Fn()>>
    // Rc<dyn Fn()> позволяет хранить замыкание, захватывающее переменные
    inner: ConflictResolutionCallbackSlot,
}

// SAFETY: В WASM среда однопоточная, поэтому мы можем "обмануть" компилятор,
// пообещав, что этот тип безопасен для передачи между потоками.
unsafe impl Send for ConflictResolutionHandle {}
unsafe impl Sync for ConflictResolutionHandle {}

impl ConflictResolutionHandle {
    pub fn new() -> Self {
        Self::default()
    }

    // Метод для установки callback-а (вызывается внутри connect_websocket)
    pub fn set_callback(&self, f: impl Fn() + 'static) {
        *self.inner.borrow_mut() = Some(Rc::new(f));
    }

    // Метод для вызова (вызывается из компонента)
    pub fn invoke(&self) {
        // Берем ссылку, чтобы не держать borrow_mut
        let callback = self.inner.borrow().clone();
        if let Some(f) = callback {
            f();
        } else {
            leptos::logging::log!("⚠️ Conflict resolution callback not set");
        }
    }
}

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
    pub x: ReadSignal<f64>,
    pub set_x: WriteSignal<f64>,
    pub y: ReadSignal<f64>,
    pub set_y: WriteSignal<f64>,
    pub last_activity: ReadSignal<f64>,
    pub set_last_activity: WriteSignal<f64>,
    pub visible: ReadSignal<bool>,
    pub set_visible: WriteSignal<bool>,
}
