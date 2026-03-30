use crate::config::Theme;
use crate::i18n::i18n::{t, t_string, use_i18n};
use leptos::ev::MouseEvent;
use leptos::prelude::*;

#[component]
pub fn WorkspaceHintCard(
    zoom_percent: Signal<i32>,
    on_close: Callback<()>,
    theme: Theme,
) -> impl IntoView {
    let i18n = use_i18n();

    view! {
        <div
            on:mousedown=move |event: MouseEvent| event.stop_propagation()
            on:click=move |event: MouseEvent| event.stop_propagation()
            style=format!(
                "position: absolute; right: 1rem; bottom: 1rem; display: flex; flex-direction: column; gap: 0.65rem; \
                 padding: 0.75rem 0.85rem; background: rgba(0,0,0,0.38); border: 1px solid {}; border-radius: 0.85rem; \
                 color: {}; font-size: 0.78rem; backdrop-filter: blur(8px); z-index: 6; width: min(26rem, calc(100vw - 2rem)); \
                 box-shadow: 0 12px 32px rgba(0,0,0,0.18);",
                theme.ui_border, theme.ui_text_secondary
            )
        >
            <div style="display: flex; align-items: center; justify-content: space-between; gap: 0.75rem;">
                <div style=format!("font-size: 0.9rem; font-weight: 700; color: {};", theme.ui_text_primary)>
                    {t!(i18n, scene_board.hint_title)}
                </div>
                <button
                    on:mousedown=move |event: MouseEvent| {
                        event.prevent_default();
                        event.stop_propagation();
                        on_close.run(());
                    }
                    style=format!(
                        "padding: 0.2rem 0.5rem; border: none; border-radius: 0.45rem; background: {}; color: {}; cursor: pointer; font-size: 0.9rem; line-height: 1;",
                        theme.ui_bg_secondary, theme.ui_text_primary
                    )
                >
                    "x"
                </button>
            </div>

            <div style="display: flex; flex-wrap: wrap; gap: 0.55rem 0.75rem; align-items: center;">
                <span>{move || format!("{}: {}%", t_string!(i18n, scene_board.zoom_label), zoom_percent.get())}</span>
                <span>{t!(i18n, scene_board.camera_pan)}</span>
                <span>{t!(i18n, scene_board.token_drag)}</span>
                <span>{t!(i18n, scene_board.token_free_move)}</span>
                <span>{t!(i18n, scene_board.board_select)}</span>
                <span>{t!(i18n, scene_board.board_drag)}</span>
                <span>{t!(i18n, scene_board.token_place)}</span>
                <span>{t!(i18n, scene_board.inactive_blur)}</span>
            </div>
        </div>
    }
}
