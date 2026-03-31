// Geometric helpers for the scene board: viewport utilities, hit-tests,
// scene/token mutations, and scene-position snapping.

use super::interaction_state::{
    BOARD_NOTE_EDIT_PADDING_PX, BOARD_NOTE_RESIZE_HANDLE_PX, SceneLayout,
};
use super::model::{
    BOARD_HANDLE_GAP_PX, BOARD_HANDLE_HEIGHT_PX, BOARD_HANDLE_MAX_WIDTH_PX, SNAP_THRESHOLD_PX,
    clamp_token_position, point_inside_rect, token_rect, workspace_board_metrics,
};
use crate::components::websocket::{StoredTokenLibraryItem, WsSender};
use leptos::html;
use leptos::prelude::*;
use shared::events::{ClientEvent, NotePayload, Scene, Token};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// SceneLayout geometry accessors
// ---------------------------------------------------------------------------

impl SceneLayout {
    pub fn center_x(&self) -> f64 {
        f64::from(self.scene.workspace_x)
    }
    pub fn center_y(&self) -> f64 {
        f64::from(self.scene.workspace_y)
    }
    pub fn left(&self) -> f64 {
        self.center_x() - self.board_width / 2.0
    }
    pub fn right(&self) -> f64 {
        self.center_x() + self.board_width / 2.0
    }
    pub fn top(&self) -> f64 {
        self.center_y() - self.board_height / 2.0
    }
    pub fn bottom(&self) -> f64 {
        self.center_y() + self.board_height / 2.0
    }
    pub fn handle_width(&self) -> f64 {
        self.board_width.min(BOARD_HANDLE_MAX_WIDTH_PX)
    }
    pub fn handle_left(&self) -> f64 {
        self.center_x() - self.handle_width() / 2.0
    }
    pub fn handle_top(&self) -> f64 {
        self.top() - BOARD_HANDLE_GAP_PX - BOARD_HANDLE_HEIGHT_PX
    }
}

// ---------------------------------------------------------------------------
// Viewport utilities
// ---------------------------------------------------------------------------

pub fn viewport_size() -> (f64, f64) {
    let Some(window) = web_sys::window() else {
        return (1280.0, 720.0);
    };
    let w = window
        .inner_width()
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(1280.0);
    let h = window
        .inner_height()
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(720.0);
    (w, h)
}

pub fn viewport_local_point(
    viewport_ref: &NodeRef<html::Div>,
    client_x: i32,
    client_y: i32,
) -> Option<(f64, f64)> {
    let vp = viewport_ref.get()?;
    let rect = vp.get_bounding_client_rect();
    let x = (f64::from(client_x) - rect.left()).clamp(0.0, rect.width());
    let y = (f64::from(client_y) - rect.top()).clamp(0.0, rect.height());
    Some((x, y))
}

// ---------------------------------------------------------------------------
// Scene layout construction
// ---------------------------------------------------------------------------

pub fn build_scene_layouts(scenes: &[Scene]) -> Vec<SceneLayout> {
    scenes
        .iter()
        .cloned()
        .map(|scene| {
            let (cell_size, board_width, board_height) =
                workspace_board_metrics(scene.grid.columns, scene.grid.rows);
            SceneLayout {
                scene,
                cell_size,
                board_width,
                board_height,
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Spatial hit-tests
// ---------------------------------------------------------------------------

pub fn point_inside_board(layout: &SceneLayout, wx: f64, wy: f64) -> bool {
    point_inside_rect(
        wx,
        wy,
        layout.left(),
        layout.top(),
        layout.board_width,
        layout.board_height,
    )
}

pub fn point_inside_handle(layout: &SceneLayout, wx: f64, wy: f64) -> bool {
    point_inside_rect(
        wx,
        wy,
        layout.handle_left(),
        layout.handle_top(),
        layout.handle_width(),
        BOARD_HANDLE_HEIGHT_PX,
    )
}

pub fn board_note_hit(notes: &[NotePayload], wx: f64, wy: f64) -> Option<NotePayload> {
    notes.iter().rev().find_map(|note| {
        let position = note.board_position.as_ref()?;
        point_inside_rect(
            wx,
            wy,
            position.world_x,
            position.world_y,
            note.board_style.width_px,
            note.board_style.height_px,
        )
        .then(|| note.clone())
    })
}

pub fn point_inside_board_note_content(note: &NotePayload, wx: f64, wy: f64) -> bool {
    let Some(position) = note.board_position.as_ref() else {
        return false;
    };
    let inner_left = position.world_x + BOARD_NOTE_EDIT_PADDING_PX;
    let inner_top = position.world_y + BOARD_NOTE_EDIT_PADDING_PX;
    let inner_width = (note.board_style.width_px - BOARD_NOTE_EDIT_PADDING_PX * 2.0).max(0.0);
    let inner_height = (note.board_style.height_px - BOARD_NOTE_EDIT_PADDING_PX * 2.0).max(0.0);
    point_inside_rect(wx, wy, inner_left, inner_top, inner_width, inner_height)
}

/// Returns true when the point is inside the bottom-right resize handle of a board note.
#[allow(dead_code)]
pub fn point_inside_board_note_resize_handle(note: &NotePayload, wx: f64, wy: f64) -> bool {
    let Some(position) = note.board_position.as_ref() else {
        return false;
    };
    let handle_left =
        position.world_x + note.board_style.width_px - BOARD_NOTE_RESIZE_HANDLE_PX;
    let handle_top =
        position.world_y + note.board_style.height_px - BOARD_NOTE_RESIZE_HANDLE_PX;
    point_inside_rect(
        wx,
        wy,
        handle_left,
        handle_top,
        BOARD_NOTE_RESIZE_HANDLE_PX,
        BOARD_NOTE_RESIZE_HANDLE_PX,
    )
}

pub fn token_hit(layout: &SceneLayout, wx: f64, wy: f64) -> Option<Token> {
    layout
        .scene
        .tokens
        .iter()
        .rev()
        .find(|token| {
            let (left, top, width, height) = token_rect(
                layout.left(),
                layout.top(),
                layout.cell_size,
                token.x,
                token.y,
                token.width_cells,
                token.height_cells,
            );
            point_inside_rect(wx, wy, left, top, width, height)
        })
        .cloned()
}

pub fn clamp_to_layout(wx: f64, wy: f64, layout: &SceneLayout) -> (f64, f64) {
    (
        wx.clamp(layout.left(), layout.right()),
        wy.clamp(layout.top(), layout.bottom()),
    )
}

// ---------------------------------------------------------------------------
// Scene-position snapping
// ---------------------------------------------------------------------------

pub fn snap_scene_position(
    candidate_x: f64,
    candidate_y: f64,
    dragged_width: f64,
    dragged_height: f64,
    other_layouts: &[SceneLayout],
) -> (f64, f64) {
    let mut best = (candidate_x, candidate_y);
    let mut best_score = f64::MAX;

    for layout in other_layouts {
        let h_targets = [
            layout.center_y(),
            layout.top() + dragged_height / 2.0,
            layout.bottom() - dragged_height / 2.0,
        ];
        let v_targets = [
            layout.center_x(),
            layout.left() + dragged_width / 2.0,
            layout.right() - dragged_width / 2.0,
        ];

        let neighbor_targets = [
            (layout.right() + dragged_width / 2.0, h_targets[0]),
            (layout.right() + dragged_width / 2.0, h_targets[1]),
            (layout.right() + dragged_width / 2.0, h_targets[2]),
            (layout.left() - dragged_width / 2.0, h_targets[0]),
            (layout.left() - dragged_width / 2.0, h_targets[1]),
            (layout.left() - dragged_width / 2.0, h_targets[2]),
            (v_targets[0], layout.bottom() + dragged_height / 2.0),
            (v_targets[1], layout.bottom() + dragged_height / 2.0),
            (v_targets[2], layout.bottom() + dragged_height / 2.0),
            (v_targets[0], layout.top() - dragged_height / 2.0),
            (v_targets[1], layout.top() - dragged_height / 2.0),
            (v_targets[2], layout.top() - dragged_height / 2.0),
        ];

        for (tx, ty) in neighbor_targets {
            let dx = (candidate_x - tx).abs();
            let dy = (candidate_y - ty).abs();
            if dx <= SNAP_THRESHOLD_PX && dy <= SNAP_THRESHOLD_PX {
                let score = dx + dy;
                if score < best_score {
                    best_score = score;
                    best = (tx, ty);
                }
            }
        }
    }

    best
}

// ---------------------------------------------------------------------------
// Scene / token mutations
// ---------------------------------------------------------------------------

pub fn update_scene_position(scenes: RwSignal<Vec<Scene>>, id: &str, x: f64, y: f64) {
    scenes.update(|items| {
        if let Some(scene) = items.iter_mut().find(|s| s.id == id) {
            scene.workspace_x = x as f32;
            scene.workspace_y = y as f32;
        }
    });
}

pub fn update_token_position(scenes: RwSignal<Vec<Scene>>, id: &str, x: f32, y: f32) {
    scenes.update(|items| {
        for scene in items {
            if let Some(token) = scene.tokens.iter_mut().find(|token| token.id == id) {
                token.x = x;
                token.y = y;
                break;
            }
        }
    });
}

pub fn place_library_token(
    scenes: RwSignal<Vec<Scene>>,
    scene_id: &str,
    token: &StoredTokenLibraryItem,
    x: f32,
    y: f32,
) -> Option<Scene> {
    let mut updated_scene = None::<Scene>;
    scenes.update(|items| {
        let Some(scene) = items.iter_mut().find(|scene| scene.id == scene_id) else {
            return;
        };
        scene.tokens.push(Token {
            id: Uuid::new_v4().to_string(),
            name: token.name.clone(),
            image: token.image.clone(),
            x,
            y,
            width_cells: token.width_cells,
            height_cells: token.height_cells,
        });
        updated_scene = Some(scene.clone());
    });
    updated_scene
}

pub fn remove_token_from_scene(
    scenes: RwSignal<Vec<Scene>>,
    scene_id: &str,
    token_id: &str,
) -> Option<Scene> {
    let mut updated_scene = None::<Scene>;
    scenes.update(|items| {
        let Some(scene) = items.iter_mut().find(|scene| scene.id == scene_id) else {
            return;
        };
        let original_len = scene.tokens.len();
        scene.tokens.retain(|token| token.id != token_id);
        if scene.tokens.len() != original_len {
            updated_scene = Some(scene.clone());
        }
    });
    updated_scene
}

pub fn update_token_details(
    scenes: RwSignal<Vec<Scene>>,
    scene_id: &str,
    token_id: &str,
    name: &str,
    width_cells: u16,
    height_cells: u16,
) -> Option<Scene> {
    let mut updated_scene = None::<Scene>;
    scenes.update(|items| {
        let Some(scene) = items.iter_mut().find(|scene| scene.id == scene_id) else {
            return;
        };
        let columns = scene.grid.columns;
        let rows = scene.grid.rows;
        let Some(token) = scene.tokens.iter_mut().find(|token| token.id == token_id) else {
            return;
        };
        token.name = name.to_string();
        token.width_cells = width_cells;
        token.height_cells = height_cells;
        let (x, y) =
            clamp_token_position(token.x, token.y, columns, rows, width_cells, height_cells);
        token.x = x;
        token.y = y;
        updated_scene = Some(scene.clone());
    });
    updated_scene
}

pub fn sort_token_library_items(items: &mut [StoredTokenLibraryItem]) {
    items.sort_by(|left, right| {
        left.name
            .to_lowercase()
            .cmp(&right.name.to_lowercase())
            .then_with(|| left.id.cmp(&right.id))
    });
}

/// Thin wrapper: sends a WS event if the sender is connected.
pub fn send_event(ws_sender: &ReadSignal<Option<WsSender>>, event: ClientEvent) {
    if let Some(sender) = ws_sender.get_untracked() {
        let _ = sender.try_send_event(event);
    }
}
