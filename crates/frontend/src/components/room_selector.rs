use crate::config::Theme;
use crate::i18n::i18n::{t, t_string, use_i18n};
use leptos::ev::SubmitEvent;
use leptos::prelude::*;

#[component]
pub fn RoomSelector(
    on_room_selected: Callback<String>, // room_id
    theme: Theme,
) -> impl IntoView {
    let i18n = use_i18n();

    // Загружаем последнюю комнату из localStorage
    let last_room = web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|s| s.get_item("last_room_id").ok().flatten())
        .unwrap_or_default();

    let (room_id, set_room_id) = signal(last_room);
    let (error_message, set_error_message) = signal(Option::<String>::None);

    let on_submit = move |ev: SubmitEvent| {
        ev.prevent_default();

        let room_val = room_id.get();
        if room_val.is_empty() {
            set_error_message.set(Some(t_string!(i18n, auth.room.error_empty).to_string()));
            return;
        }

        // Сохраняем в localStorage
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                let _ = storage.set_item("last_room_id", &room_val);
            }
        }

        on_room_selected.run(room_val);
    };

    view! {
        <div style="display: flex; flex-direction: column; align-items: center; justify-content: center; min-height: 100vh; padding: 1.25rem;">
            <div style=format!("background: {}; padding: 2.5rem; border-radius: 0.625rem; max-width: 25rem; width: 100%;", theme.ui_bg_primary)>
                <h1 style=format!("color: {}; text-align: center; margin-bottom: 1.875rem;", theme.ui_text_primary)>{t!(i18n, auth.room.title)}</h1>

                <form on:submit=on_submit style="display: flex; flex-direction: column; gap: 1.25rem;">
                    <div style="display: flex; flex-direction: column; gap: 0.5rem;">
                        <label style=format!("color: {};", theme.ui_text_primary)>{t!(i18n, auth.room.room_id)}</label>
                        <input
                            type="text"
                            value=move || room_id.get()
                            on:input=move |ev| set_room_id.set(event_target_value(&ev))
                            placeholder=move || t_string!(i18n, auth.room.room_id).to_string()
                            style=format!("padding: 0.75rem; border-radius: 0.3125rem; border: 0.0625rem solid {}; background: {}; color: {}; font-size: 1rem;", theme.ui_border, theme.ui_bg_secondary, theme.ui_text_primary)
                        />
                    </div>

                    <Show when=move || error_message.get().is_some()>
                        <div style=format!("padding: 0.75rem; background: {}; border: 0.0625rem solid {}; border-radius: 0.3125rem; color: {};", theme.ui_bg_secondary, theme.ui_button_danger, theme.ui_button_danger)>
                            {move || error_message.get().unwrap_or_default()}
                        </div>
                    </Show>

                    <button
                        type="submit"
                        style=format!("padding: 0.75rem; background: {}; color: {}; border: none; border-radius: 0.3125rem; font-size: 1rem; cursor: pointer; font-weight: bold;", theme.ui_button_primary, theme.ui_text_primary)
                    >
                        {t!(i18n, auth.room.button)}
                    </button>
                </form>
            </div>
        </div>
    }
}
