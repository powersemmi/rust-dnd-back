use crate::components::websocket::{storage, utils};
use leptos::prelude::*;
use shared::events::{
    RoomState, Scene, SceneActivatePayload, SceneCreatePayload, SceneDeletePayload,
    SceneUpdatePayload, TokenMovePayload,
};
use std::cell::RefCell;
use std::rc::Rc;

use super::HandlerContext;

const MAX_SCENES_PER_ROOM: usize = 50;
const TOKEN_POSITION_EPSILON: f32 = 0.001;

fn sync_scene_signals(
    room_state: &Rc<RefCell<RoomState>>,
    scenes_signal: RwSignal<Vec<Scene>>,
    active_scene_id_signal: RwSignal<Option<String>>,
) {
    let state = room_state.borrow();
    scenes_signal.set(state.scenes.clone());
    active_scene_id_signal.set(state.active_scene_id.clone());
}

pub fn handle_scene_create(payload: SceneCreatePayload, ctx: &HandlerContext<'_>) {
    let current_ver = {
        let mut state = ctx.room_state.borrow_mut();
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

    *ctx.local_version.borrow_mut() = current_ver;
    if payload.actor != ctx.my_username {
        *ctx.last_synced_version.borrow_mut() = current_ver;
    }

    sync_scene_signals(
        ctx.room_state,
        ctx.scenes_signal,
        ctx.active_scene_id_signal,
    );
    storage::save_state_in_background(ctx.room_name, &ctx.room_state.borrow());
    utils::log_event(
        ctx.state_events,
        current_ver,
        "SCENE_CREATE",
        &format!("{} created scene '{}'", payload.actor, payload.scene.name),
    );
}

pub fn handle_scene_update(payload: SceneUpdatePayload, ctx: &HandlerContext<'_>) {
    let current_ver = {
        let mut state = ctx.room_state.borrow_mut();
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

    *ctx.local_version.borrow_mut() = current_ver;
    if payload.actor != ctx.my_username {
        *ctx.last_synced_version.borrow_mut() = current_ver;
    }

    sync_scene_signals(
        ctx.room_state,
        ctx.scenes_signal,
        ctx.active_scene_id_signal,
    );
    storage::save_state_in_background(ctx.room_name, &ctx.room_state.borrow());
    utils::log_event(
        ctx.state_events,
        current_ver,
        "SCENE_UPDATE",
        &format!("{} updated scene '{}'", payload.actor, payload.scene.name),
    );
}

pub fn handle_scene_delete(payload: SceneDeletePayload, ctx: &HandlerContext<'_>) {
    let (current_ver, deleted_name) = {
        let mut state = ctx.room_state.borrow_mut();
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

    *ctx.local_version.borrow_mut() = current_ver;
    if payload.actor != ctx.my_username {
        *ctx.last_synced_version.borrow_mut() = current_ver;
    }

    sync_scene_signals(
        ctx.room_state,
        ctx.scenes_signal,
        ctx.active_scene_id_signal,
    );
    storage::save_state_in_background(ctx.room_name, &ctx.room_state.borrow());
    utils::log_event(
        ctx.state_events,
        current_ver,
        "SCENE_DELETE",
        &format!("{} deleted scene '{}'", payload.actor, deleted_name),
    );
}

pub fn handle_scene_activate(payload: SceneActivatePayload, ctx: &HandlerContext<'_>) {
    let (current_ver, activated_name) = {
        let mut state = ctx.room_state.borrow_mut();
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

    *ctx.local_version.borrow_mut() = current_ver;
    if payload.actor != ctx.my_username {
        *ctx.last_synced_version.borrow_mut() = current_ver;
    }

    sync_scene_signals(
        ctx.room_state,
        ctx.scenes_signal,
        ctx.active_scene_id_signal,
    );
    storage::save_state_in_background(ctx.room_name, &ctx.room_state.borrow());
    utils::log_event(
        ctx.state_events,
        current_ver,
        "SCENE_ACTIVATE",
        &format!("{} activated scene '{}'", payload.actor, activated_name),
    );
}

pub fn handle_token_move(payload: TokenMovePayload, ctx: &HandlerContext<'_>) {
    let (current_ver, moved_token_name) = {
        let mut state = ctx.room_state.borrow_mut();
        let mut moved_token_name = None::<String>;

        for scene in &mut state.scenes {
            let Some(token) = scene
                .tokens
                .iter_mut()
                .find(|token| token.id == payload.token_id)
            else {
                continue;
            };

            if (token.x - payload.x).abs() < TOKEN_POSITION_EPSILON
                && (token.y - payload.y).abs() < TOKEN_POSITION_EPSILON
            {
                return;
            }

            token.x = payload.x;
            token.y = payload.y;
            moved_token_name = Some(token.name.clone());
            break;
        }

        let Some(moved_token_name) = moved_token_name else {
            return;
        };

        state.commit_changes();
        (state.version, moved_token_name)
    };

    *ctx.local_version.borrow_mut() = current_ver;
    if payload.actor != ctx.my_username {
        *ctx.last_synced_version.borrow_mut() = current_ver;
    }

    sync_scene_signals(
        ctx.room_state,
        ctx.scenes_signal,
        ctx.active_scene_id_signal,
    );
    storage::save_state_in_background(ctx.room_name, &ctx.room_state.borrow());
    utils::log_event(
        ctx.state_events,
        current_ver,
        "TOKEN_MOVE",
        &format!("{} moved token '{}'", payload.actor, moved_token_name),
    );
}
