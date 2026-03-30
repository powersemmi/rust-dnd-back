use crate::components::statistics::StateEvent;
use crate::components::websocket::{storage, utils};
use leptos::prelude::*;
use shared::events::{
    RoomState, Scene, SceneActivatePayload, SceneCreatePayload, SceneDeletePayload,
    SceneUpdatePayload,
};
use std::cell::RefCell;
use std::rc::Rc;

const MAX_SCENES_PER_ROOM: usize = 50;

fn sync_scene_signals(
    room_state: &Rc<RefCell<RoomState>>,
    scenes_signal: RwSignal<Vec<Scene>>,
    active_scene_id_signal: RwSignal<Option<String>>,
) {
    let state = room_state.borrow();
    scenes_signal.set(state.scenes.clone());
    active_scene_id_signal.set(state.active_scene_id.clone());
}

#[allow(clippy::too_many_arguments)]
pub fn handle_scene_create(
    payload: SceneCreatePayload,
    room_state: &Rc<RefCell<RoomState>>,
    local_version: &Rc<RefCell<u64>>,
    last_synced_version: &Rc<RefCell<u64>>,
    my_username: &str,
    room_name: &str,
    scenes_signal: RwSignal<Vec<Scene>>,
    active_scene_id_signal: RwSignal<Option<String>>,
    state_events: RwSignal<Vec<StateEvent>>,
) {
    let current_ver = {
        let mut state = room_state.borrow_mut();
        if state
            .scenes
            .iter()
            .any(|scene| scene.id == payload.scene.id)
            || state.scenes.len() >= MAX_SCENES_PER_ROOM
        {
            return;
        }

        if state.active_scene_id.is_none() {
            state.active_scene_id = Some(payload.scene.id.clone());
        }

        state.scenes.push(payload.scene.clone());
        state.commit_changes();
        state.version
    };

    *local_version.borrow_mut() = current_ver;
    if payload.actor != my_username {
        *last_synced_version.borrow_mut() = current_ver;
    }

    sync_scene_signals(room_state, scenes_signal, active_scene_id_signal);
    storage::save_state_in_background(room_name, &room_state.borrow());
    utils::log_event(
        state_events,
        current_ver,
        "SCENE_CREATE",
        &format!("{} created scene '{}'", payload.actor, payload.scene.name),
    );
}

#[allow(clippy::too_many_arguments)]
pub fn handle_scene_update(
    payload: SceneUpdatePayload,
    room_state: &Rc<RefCell<RoomState>>,
    local_version: &Rc<RefCell<u64>>,
    last_synced_version: &Rc<RefCell<u64>>,
    my_username: &str,
    room_name: &str,
    scenes_signal: RwSignal<Vec<Scene>>,
    active_scene_id_signal: RwSignal<Option<String>>,
    state_events: RwSignal<Vec<StateEvent>>,
) {
    let current_ver = {
        let mut state = room_state.borrow_mut();
        let Some(scene) = state
            .scenes
            .iter_mut()
            .find(|scene| scene.id == payload.scene.id)
        else {
            return;
        };

        *scene = payload.scene.clone();
        state.commit_changes();
        state.version
    };

    *local_version.borrow_mut() = current_ver;
    if payload.actor != my_username {
        *last_synced_version.borrow_mut() = current_ver;
    }

    sync_scene_signals(room_state, scenes_signal, active_scene_id_signal);
    storage::save_state_in_background(room_name, &room_state.borrow());
    utils::log_event(
        state_events,
        current_ver,
        "SCENE_UPDATE",
        &format!("{} updated scene '{}'", payload.actor, payload.scene.name),
    );
}

#[allow(clippy::too_many_arguments)]
pub fn handle_scene_delete(
    payload: SceneDeletePayload,
    room_state: &Rc<RefCell<RoomState>>,
    local_version: &Rc<RefCell<u64>>,
    last_synced_version: &Rc<RefCell<u64>>,
    my_username: &str,
    room_name: &str,
    scenes_signal: RwSignal<Vec<Scene>>,
    active_scene_id_signal: RwSignal<Option<String>>,
    state_events: RwSignal<Vec<StateEvent>>,
) {
    let (current_ver, deleted_name) = {
        let mut state = room_state.borrow_mut();
        let Some(index) = state
            .scenes
            .iter()
            .position(|scene| scene.id == payload.scene_id)
        else {
            return;
        };

        let deleted_scene = state.scenes.remove(index);

        if state.active_scene_id.as_deref() == Some(payload.scene_id.as_str()) {
            state.active_scene_id = state.scenes.first().map(|scene| scene.id.clone());
        }

        state.commit_changes();
        (state.version, deleted_scene.name)
    };

    *local_version.borrow_mut() = current_ver;
    if payload.actor != my_username {
        *last_synced_version.borrow_mut() = current_ver;
    }

    sync_scene_signals(room_state, scenes_signal, active_scene_id_signal);
    storage::save_state_in_background(room_name, &room_state.borrow());
    utils::log_event(
        state_events,
        current_ver,
        "SCENE_DELETE",
        &format!("{} deleted scene '{}'", payload.actor, deleted_name),
    );
}

#[allow(clippy::too_many_arguments)]
pub fn handle_scene_activate(
    payload: SceneActivatePayload,
    room_state: &Rc<RefCell<RoomState>>,
    local_version: &Rc<RefCell<u64>>,
    last_synced_version: &Rc<RefCell<u64>>,
    my_username: &str,
    room_name: &str,
    scenes_signal: RwSignal<Vec<Scene>>,
    active_scene_id_signal: RwSignal<Option<String>>,
    state_events: RwSignal<Vec<StateEvent>>,
) {
    let (current_ver, activated_name) = {
        let mut state = room_state.borrow_mut();
        let Some(scene_name) = state
            .scenes
            .iter()
            .find(|scene| scene.id == payload.scene_id)
            .map(|scene| scene.name.clone())
        else {
            return;
        };

        if state.active_scene_id.as_deref() == Some(payload.scene_id.as_str()) {
            return;
        }

        state.active_scene_id = Some(payload.scene_id.clone());
        state.commit_changes();
        (state.version, scene_name)
    };

    *local_version.borrow_mut() = current_ver;
    if payload.actor != my_username {
        *last_synced_version.borrow_mut() = current_ver;
    }

    sync_scene_signals(room_state, scenes_signal, active_scene_id_signal);
    storage::save_state_in_background(room_name, &room_state.borrow());
    utils::log_event(
        state_events,
        current_ver,
        "SCENE_ACTIVATE",
        &format!("{} activated scene '{}'", payload.actor, activated_name),
    );
}
