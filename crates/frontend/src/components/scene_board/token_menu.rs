use crate::config::Theme;
use crate::i18n::i18n::{t, use_i18n};
use leptos::ev::MouseEvent;
use leptos::portal::Portal;
use leptos::prelude::*;

#[component]
pub fn SceneTokenMenu(
    token_name: String,
    screen_x: f64,
    screen_y: f64,
    on_edit: Callback<()>,
    on_save_to_library: Callback<()>,
    on_delete: Callback<()>,
    on_close: Callback<()>,
    theme: Theme,
) -> impl IntoView {
    let i18n = use_i18n();
    let token_name_text = RwSignal::new(token_name);

    view! {
        <Portal>
            <div
                on:mousedown=move |_| on_close.run(())
                style="position: fixed; inset: 0; z-index: 2100;"
            />
            <div
                on:mousedown=move |event: MouseEvent| event.stop_propagation()
                on:click=move |event: MouseEvent| event.stop_propagation()
                style=format!(
                    "position: fixed; left: {:.2}px; top: {:.2}px; min-width: 12rem; max-width: 18rem; \
                     z-index: 2101; padding: 0.7rem; border: 1px solid {}; border-radius: 0.9rem; \
                     background: rgba(15, 23, 42, 0.94); box-shadow: 0 22px 48px rgba(0,0,0,0.34); \
                     backdrop-filter: blur(12px);",
                    screen_x, screen_y, theme.ui_border
                )
            >
                <div style=format!("font-size: 0.78rem; color: {}; margin-bottom: 0.3rem;", theme.ui_text_secondary)>
                    {t!(i18n, tokens.menu_label)}
                </div>
                <div style=format!("font-size: 0.9rem; font-weight: 700; color: {}; margin-bottom: 0.7rem; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;", theme.ui_text_primary)>
                    {move || token_name_text.get()}
                </div>
                <button
                    on:mousedown=move |event: MouseEvent| {
                        event.prevent_default();
                        event.stop_propagation();
                        on_edit.run(());
                    }
                    style=format!(
                        "width: 100%; margin-bottom: 0.45rem; padding: 0.6rem 0.75rem; border: 1px solid {}; \
                         border-radius: 0.7rem; background: rgba(255,255,255,0.04); color: {}; cursor: pointer; \
                         text-align: left; font-weight: 700;",
                        theme.ui_border, theme.ui_text_primary
                    )
                >
                    {t!(i18n, tokens.edit_button)}
                </button>
                <button
                    on:mousedown=move |event: MouseEvent| {
                        event.prevent_default();
                        event.stop_propagation();
                        on_save_to_library.run(());
                    }
                    style=format!(
                        "width: 100%; margin-bottom: 0.45rem; padding: 0.6rem 0.75rem; border: 1px solid {}; \
                         border-radius: 0.7rem; background: rgba(255,255,255,0.04); color: {}; cursor: pointer; \
                         text-align: left; font-weight: 700;",
                        theme.ui_border, theme.ui_text_primary
                    )
                >
                    {t!(i18n, tokens.save_to_library)}
                </button>
                <button
                    on:mousedown=move |event: MouseEvent| {
                        event.prevent_default();
                        event.stop_propagation();
                        on_delete.run(());
                    }
                    style="width: 100%; padding: 0.6rem 0.75rem; border: none; border-radius: 0.7rem; background: rgba(220, 38, 38, 0.18); color: #fecaca; cursor: pointer; text-align: left; font-weight: 700;"
                >
                    {t!(i18n, tokens.delete_from_scene)}
                </button>
            </div>
        </Portal>
    }
}
