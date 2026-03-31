// Full-screen background fit editor modal for a scene.
// Shown when the user wants to adjust scale/position/rotation of a scene background.

use super::model::{
    MAX_BACKGROUND_ROTATION_DEG, MAX_BACKGROUND_SCALE, MIN_BACKGROUND_ROTATION_DEG,
    MIN_BACKGROUND_SCALE, fit_preview_layout,
};
use super::view_model::ScenesWindowViewModel;
use crate::components::websocket::{FileTransferStage, FileTransferState};
use crate::config::Theme;
use crate::i18n::i18n::{t, t_string, use_i18n};
use leptos::ev::MouseEvent;
use leptos::prelude::*;
use shared::events::Scene;

const TITLE_FONT_SIZE: &str = "clamp(1rem, 0.95rem + 0.22vw, 1.18rem)";
const BODY_FONT_SIZE: &str = "clamp(0.9rem, 0.87rem + 0.12vw, 0.98rem)";
const META_FONT_SIZE: &str = "clamp(0.74rem, 0.71rem + 0.12vw, 0.82rem)";
const BUTTON_FONT_SIZE: &str = "clamp(0.84rem, 0.81rem + 0.12vw, 0.94rem)";

fn viewport_size() -> (f64, f64) {
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

/// Full-screen overlay for adjusting a scene background's scale, position, and rotation.
/// `on_save` is called when the user clicks Save; the caller is responsible for persisting.
#[component]
pub fn BackgroundFitEditor(
    vm: ScenesWindowViewModel,
    #[prop(into)] scenes: RwSignal<Vec<Scene>>,
    file_transfer: FileTransferState,
    #[prop(into)] on_save: Callback<()>,
    theme: Theme,
) -> impl IntoView {
    let i18n = use_i18n();

    view! {
        <div style="position: fixed; inset: 0; background: rgba(5,10,18,0.72); backdrop-filter: blur(6px); z-index: 2200; display: flex; align-items: stretch; justify-content: stretch;">
            <div style="flex: 1; display: flex; align-items: stretch; justify-content: stretch;"
                on:click=move |e: MouseEvent| e.stop_propagation()
            >
                // Preview area
                <div style="flex: 1; min-width: 0; padding: 1.5rem; display: flex; flex-direction: column; gap: 1rem;">
                    <div style="display: flex; justify-content: space-between; gap: 1rem; align-items: flex-end; flex-wrap: wrap;">
                        <div style="min-width: 0;">
                            <div style=format!(
                                "color: {}; font-size: {}; font-weight: 800; line-height: 1.15;",
                                theme.ui_text_primary, TITLE_FONT_SIZE
                            )>
                                {move || t!(i18n, scenes.background_fit_title)}
                            </div>
                            <div style=format!(
                                "color: {}; font-size: {}; margin-top: 0.35rem; line-height: 1.45;",
                                theme.ui_text_secondary, BODY_FONT_SIZE
                            )>
                                {move || t!(i18n, scenes.background_fit_live_preview_hint)}
                            </div>
                        </div>
                        <div style=format!(
                            "color: {}; font-size: {};",
                            theme.ui_text_muted, META_FONT_SIZE
                        )>
                            {move || {
                                let cols = vm.draft_columns.get().parse::<u16>().ok().filter(|v| (1..=200).contains(v));
                                let rows = vm.draft_rows.get().parse::<u16>().ok().filter(|v| (1..=200).contains(v));
                                let feet = vm.draft_cell_size_feet.get().parse::<u16>().ok().filter(|v| (1..=100).contains(v));
                                match (cols, rows, feet) {
                                    (Some(c), Some(r), Some(f)) => format!("{} x {} cells · {} ft/cell", c, r, f),
                                    _ => t_string!(i18n, scenes.error_invalid_grid).to_string(),
                                }
                            }}
                        </div>
                    </div>

                    <div style=format!("flex: 1; min-height: 0; border: 0.0625rem solid {}; border-radius: 1rem; background: radial-gradient(circle at top, rgba(255,255,255,0.07), transparent 30%), linear-gradient(180deg, rgba(255,255,255,0.03), rgba(0,0,0,0.18)), {}; box-shadow: inset 0 0 0 0.0625rem rgba(255,255,255,0.04); display: flex; align-items: center; justify-content: center; overflow: hidden; position: relative;", theme.ui_border, theme.ui_bg_primary)>
                        {move || {
                            let cols = vm.draft_columns.get().parse::<u16>().ok().filter(|v| (1..=200).contains(v));
                            let rows = vm.draft_rows.get().parse::<u16>().ok().filter(|v| (1..=200).contains(v));
                            let (Some(columns), Some(rows)) = (cols, rows) else {
                                return view! {
                                    <div style=format!(
                                        "color: {}; font-size: {};",
                                        theme.ui_text_muted, BODY_FONT_SIZE
                                    )>
                                        {t!(i18n, scenes.error_invalid_grid)}
                                    </div>
                                }.into_any();
                            };

                            let (vw, vh) = viewport_size();
                            let board = fit_preview_layout(columns, rows, vw, vh);
                            let stroke_minor = "rgba(255,255,255,0.18)";
                            let stroke_major = "rgba(255,255,255,0.32)";
                            let line_width = (1.0 / board.scale.max(0.35)).min(2.8);
                            let preview_url = vm.draft_background.get().and_then(|f| {
                                if f.mime_type.starts_with("image/") {
                                    file_transfer.file_urls.get().get(&f.hash).cloned()
                                } else { None }
                            });
                            let transfer_status = vm.draft_background.get().and_then(|f| {
                                file_transfer.transfer_statuses.get().get(&f.hash).cloned()
                            });

                            view! {
                                <div style=format!("position: relative; width: {:.2}px; height: {:.2}px; transform: scale({:.4}); transform-origin: center center; flex: 0 0 auto;", board.board_width, board.board_height, board.scale)>
                                    <div style=format!("position: relative; width: {:.2}px; height: {:.2}px; border: 0.125rem solid {}; border-radius: 1rem; overflow: hidden; background: linear-gradient(180deg, rgba(255,255,255,0.04), rgba(0,0,0,0.22)), {}; box-shadow: 0 1.5rem 5rem rgba(0,0,0,0.32), inset 0 0 0 0.0625rem rgba(255,255,255,0.06);", board.board_width, board.board_height, theme.ui_success, theme.ui_bg_primary)>
                                        {match preview_url {
                                            Some(url) => view! {
                                                <>
                                                    <img src=url
                                                        alt=move || vm.draft_name.get()
                                                        style=format!("position: absolute; left: 50%; top: 50%; width: {:.2}px; max-width: none; pointer-events: none; transform: translate(-50%, -50%) translate({:.2}px, {:.2}px) scale({:.4}) rotate({:.2}deg); opacity: 0.94;",
                                                            board.board_width,
                                                            vm.draft_background_offset_x.get(),
                                                            vm.draft_background_offset_y.get(),
                                                            vm.draft_background_scale.get().max(0.05),
                                                            vm.draft_background_rotation_deg.get()
                                                        )
                                                    />
                                                    <div
                                                        on:mousedown=move |e: MouseEvent| {
                                                            e.prevent_default();
                                                            e.stop_propagation();
                                                            vm.start_background_drag(e.client_x(), e.client_y(), board.scale);
                                                        }
                                                        style=move || format!("position: absolute; inset: 0; cursor: {}; background: transparent;",
                                                            if vm.is_dragging_background.get() { "grabbing" } else { "grab" }
                                                        )
                                                    />
                                                </>
                                            }.into_any(),
                                            None => match transfer_status {
                                                Some(s) if s.stage != FileTransferStage::Complete => view! {
                                                    <div style=format!(
                                                        "position: absolute; inset: 0; display: flex; align-items: center; justify-content: center; color: {}; font-size: {}; background: rgba(0,0,0,0.18);",
                                                        theme.ui_text_secondary, BODY_FONT_SIZE
                                                    )>
                                                        {format!("{} {}%", t_string!(i18n, scenes.background_status), s.progress_percent())}
                                                    </div>
                                                }.into_any(),
                                                _ => view! {
                                                    <div style=format!(
                                                        "position: absolute; inset: 0; display: flex; align-items: center; justify-content: center; color: {}; font-size: {};",
                                                        theme.ui_text_muted, BODY_FONT_SIZE
                                                    )>
                                                        {t!(i18n, scenes.background_empty)}
                                                    </div>
                                                }.into_any(),
                                            }
                                        }}
                                        <svg
                                            viewBox=format!("0 0 {:.4} {:.4}", board.board_width, board.board_height)
                                            preserveAspectRatio="none"
                                            style="position: absolute; inset: 0; width: 100%; height: 100%; pointer-events: none; shape-rendering: geometricPrecision;"
                                        >
                                            {(0..=columns).filter(|c| c % 5 != 0).map(|c| {
                                                let x = f64::from(c) * board.cell_size;
                                                view! { <line x1=format!("{x:.4}") y1="0" x2=format!("{x:.4}") y2=format!("{:.4}", board.board_height) stroke=stroke_minor stroke-width=format!("{line_width:.4}") /> }
                                            }).collect_view()}
                                            {(0..=rows).filter(|r| r % 5 != 0).map(|r| {
                                                let y = f64::from(r) * board.cell_size;
                                                view! { <line x1="0" y1=format!("{y:.4}") x2=format!("{:.4}", board.board_width) y2=format!("{y:.4}") stroke=stroke_minor stroke-width=format!("{line_width:.4}") /> }
                                            }).collect_view()}
                                            {(0..=columns).filter(|c| c % 5 == 0).map(|c| {
                                                let x = f64::from(c) * board.cell_size;
                                                view! { <line x1=format!("{x:.4}") y1="0" x2=format!("{x:.4}") y2=format!("{:.4}", board.board_height) stroke=stroke_major stroke-width=format!("{:.4}", line_width * 1.15) /> }
                                            }).collect_view()}
                                            {(0..=rows).filter(|r| r % 5 == 0).map(|r| {
                                                let y = f64::from(r) * board.cell_size;
                                                view! { <line x1="0" y1=format!("{y:.4}") x2=format!("{:.4}", board.board_width) y2=format!("{y:.4}") stroke=stroke_major stroke-width=format!("{:.4}", line_width * 1.15) /> }
                                            }).collect_view()}
                                        </svg>
                                    </div>
                                </div>
                            }.into_any()
                        }}
                    </div>
                </div>

                // Sidebar controls
                <div style=format!("width: min(34rem, 100vw); height: 100vh; background: linear-gradient(180deg, {}, {}); border-left: 0.0625rem solid {}; box-shadow: -1.5rem 0 4rem rgba(0,0,0,0.35); display: flex; flex-direction: column;", theme.ui_bg_primary, theme.ui_bg_secondary, theme.ui_border)>
                    <div style=format!("padding: 1.4rem 1.5rem 1rem; border-bottom: 0.0625rem solid {}; display: flex; justify-content: space-between; gap: 1rem; align-items: flex-start;", theme.ui_border)>
                        <div style="min-width: 0;">
                            <div style=format!(
                                "color: {}; font-size: {}; font-weight: 800; line-height: 1.15;",
                                theme.ui_text_primary, TITLE_FONT_SIZE
                            )>
                                {move || t!(i18n, scenes.background_fit_modal_title)}
                            </div>
                            <div style=format!(
                                "color: {}; font-size: {}; margin-top: 0.45rem; line-height: 1.45;",
                                theme.ui_text_secondary, BODY_FONT_SIZE
                            )>
                                {move || t!(i18n, scenes.background_fit_modal_hint)}
                            </div>
                            <div style=format!(
                                "color: {}; font-size: {}; margin-top: 0.55rem;",
                                theme.ui_text_muted, META_FONT_SIZE
                            )>
                                {move || {
                                    vm.selected_scene_id.get()
                                        .and_then(|id| scenes.get().into_iter().find(|s| s.id == id).map(|s| s.name))
                                        .unwrap_or_default()
                                }}
                            </div>
                        </div>
                        <button type="button"
                            on:click=move |_| vm.close_background_fit_editor()
                            style=format!(
                                "background: {}; border: none; color: {}; padding: 0.35rem 0.75rem; border-radius: 0.5rem; cursor: pointer; font-size: {}; font-weight: 700;",
                                theme.ui_button_danger, theme.ui_text_primary, BUTTON_FONT_SIZE
                            )
                        >
                            "x"
                        </button>
                    </div>

                    <div style="flex: 1; overflow-y: auto; padding: 1.5rem; display: flex; flex-direction: column; gap: 1rem;">
                        <div style=format!(
                            "padding: 0.9rem 1rem; border-radius: 0.75rem; background: rgba(255,255,255,0.04); border: 0.0625rem solid {}; color: {}; font-size: {}; line-height: 1.5;",
                            theme.ui_border, theme.ui_text_secondary, BODY_FONT_SIZE
                        )>
                            {move || t!(i18n, scenes.background_fit_live_preview_hint)}
                        </div>

                        <div style="display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 0.85rem;">
                            <label style=format!("color: {}; display: flex; flex-direction: column; gap: 0.45rem;", theme.ui_text_secondary)>
                                <span>{move || t!(i18n, scenes.columns_label)}</span>
                                <input type="number" min="1" max="200"
                                    prop:value=move || vm.draft_columns.get()
                                    on:input=move |ev| vm.draft_columns.set(event_target_value(&ev))
                                    style=format!(
                                        "padding: 0.75rem; background: {}; color: {}; border: 0.0625rem solid {}; border-radius: 0.5rem; font-size: {};",
                                        theme.ui_bg_secondary, theme.ui_text_primary, theme.ui_border, BODY_FONT_SIZE
                                    )
                                />
                            </label>
                            <label style=format!("color: {}; display: flex; flex-direction: column; gap: 0.45rem;", theme.ui_text_secondary)>
                                <span>{move || t!(i18n, scenes.rows_label)}</span>
                                <input type="number" min="1" max="200"
                                    prop:value=move || vm.draft_rows.get()
                                    on:input=move |ev| vm.draft_rows.set(event_target_value(&ev))
                                    style=format!(
                                        "padding: 0.75rem; background: {}; color: {}; border: 0.0625rem solid {}; border-radius: 0.5rem; font-size: {};",
                                        theme.ui_bg_secondary, theme.ui_text_primary, theme.ui_border, BODY_FONT_SIZE
                                    )
                                />
                            </label>
                        </div>

                        // Background scale
                        <label style=format!("color: {}; display: flex; flex-direction: column; gap: 0.45rem;", theme.ui_text_secondary)>
                            <span>{move || t!(i18n, scenes.background_scale_label)}</span>
                            <div style="display: flex; gap: 0.75rem; align-items: center;">
                                <input type="range" min=MIN_BACKGROUND_SCALE.to_string() max=MAX_BACKGROUND_SCALE.to_string() step="0.01"
                                    prop:value=move || format!("{:.2}", vm.draft_background_scale.get())
                                    on:input=move |ev| {
                                        if let Ok(v) = event_target_value(&ev).parse::<f32>() {
                                            vm.draft_background_scale.set(vm.clamp_background_scale(v));
                                        }
                                    }
                                    style="flex: 1;"
                                />
                                <input type="number" min=MIN_BACKGROUND_SCALE.to_string() max=MAX_BACKGROUND_SCALE.to_string() step="0.01"
                                    prop:value=move || format!("{:.2}", vm.draft_background_scale.get())
                                    on:input=move |ev| {
                                        if let Ok(v) = event_target_value(&ev).parse::<f32>() {
                                            vm.draft_background_scale.set(vm.clamp_background_scale(v));
                                        }
                                    }
                                    style=format!(
                                        "width: 6rem; padding: 0.65rem; background: {}; color: {}; border: 0.0625rem solid {}; border-radius: 0.5rem; font-size: {};",
                                        theme.ui_bg_secondary, theme.ui_text_primary, theme.ui_border, BODY_FONT_SIZE
                                    )
                                />
                            </div>
                        </label>

                        // Background position info
                        <div style=format!("padding: 0.95rem 1rem; border-radius: 0.75rem; background: rgba(255,255,255,0.03); border: 0.0625rem solid {}; display: flex; flex-direction: column; gap: 0.55rem;", theme.ui_border)>
                            <div style=format!(
                                "color: {}; font-size: {}; font-weight: 700;",
                                theme.ui_text_primary, BODY_FONT_SIZE
                            )>
                                {move || t!(i18n, scenes.background_position_label)}
                            </div>
                            <div style=format!(
                                "color: {}; font-size: {}; line-height: 1.45;",
                                theme.ui_text_secondary, BODY_FONT_SIZE
                            )>
                                {move || t!(i18n, scenes.background_fit_drag_hint)}
                            </div>
                            <div style=format!(
                                "color: {}; font-size: {};",
                                theme.ui_text_muted, META_FONT_SIZE
                            )>
                                {move || format!("X: {:.0}px | Y: {:.0}px", vm.draft_background_offset_x.get(), vm.draft_background_offset_y.get())}
                            </div>
                        </div>

                        // Background rotation
                        <label style=format!("color: {}; display: flex; flex-direction: column; gap: 0.45rem;", theme.ui_text_secondary)>
                            <span>{move || t!(i18n, scenes.background_rotation_label)}</span>
                            <div style="display: flex; gap: 0.75rem; align-items: center;">
                                <input type="range" min=MIN_BACKGROUND_ROTATION_DEG.to_string() max=MAX_BACKGROUND_ROTATION_DEG.to_string() step="1"
                                    prop:value=move || format!("{:.0}", vm.draft_background_rotation_deg.get())
                                    on:input=move |ev| {
                                        if let Ok(v) = event_target_value(&ev).parse::<f32>() {
                                            vm.draft_background_rotation_deg.set(vm.clamp_background_rotation(v));
                                        }
                                    }
                                    style="flex: 1;"
                                />
                                <input type="number" min=MIN_BACKGROUND_ROTATION_DEG.to_string() max=MAX_BACKGROUND_ROTATION_DEG.to_string() step="1"
                                    prop:value=move || format!("{:.0}", vm.draft_background_rotation_deg.get())
                                    on:input=move |ev| {
                                        if let Ok(v) = event_target_value(&ev).parse::<f32>() {
                                            vm.draft_background_rotation_deg.set(vm.clamp_background_rotation(v));
                                        }
                                    }
                                    style=format!(
                                        "width: 6rem; padding: 0.65rem; background: {}; color: {}; border: 0.0625rem solid {}; border-radius: 0.5rem; font-size: {};",
                                        theme.ui_bg_secondary, theme.ui_text_primary, theme.ui_border, BODY_FONT_SIZE
                                    )
                                />
                            </div>
                        </label>

                        // Footer buttons
                        <div style=format!("margin-top: auto; display: flex; gap: 0.75rem; justify-content: flex-end; padding-top: 0.5rem; border-top: 0.0625rem solid {}; flex-wrap: wrap;", theme.ui_border)>
                            <button type="button"
                                on:click=move |_| vm.reset_background_fit()
                                style=format!(
                                    "padding: 0.75rem 1rem; background: {}; color: {}; border: none; border-radius: 0.5rem; cursor: pointer; font-size: {};",
                                    theme.ui_bg_secondary, theme.ui_text_primary, BUTTON_FONT_SIZE
                                )
                            >
                                {move || t!(i18n, scenes.background_reset_fit_button)}
                            </button>
                            <button type="button"
                                on:click=move |_| vm.close_background_fit_editor()
                                style=format!(
                                    "padding: 0.75rem 1rem; background: {}; color: {}; border: none; border-radius: 0.5rem; cursor: pointer; font-size: {};",
                                    theme.ui_bg_secondary, theme.ui_text_primary, BUTTON_FONT_SIZE
                                )
                            >
                                {move || t!(i18n, scenes.background_fit_modal_close_button)}
                            </button>
                            <button type="button"
                                on:click=move |_| {
                                    on_save.run(());
                                    vm.close_background_fit_editor();
                                }
                                style=format!(
                                    "padding: 0.75rem 1rem; background: {}; color: {}; border: none; border-radius: 0.5rem; cursor: pointer; font-weight: 700; font-size: {};",
                                    theme.ui_button_primary, theme.ui_text_primary, BUTTON_FONT_SIZE
                                )
                            >
                                {move || t!(i18n, scenes.background_fit_modal_save_button)}
                            </button>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}
