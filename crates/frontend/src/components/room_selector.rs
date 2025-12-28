use crate::config::Theme;
use leptos::prelude::*;
use leptos::ev::SubmitEvent;

#[component]
pub fn RoomSelector(
    on_room_selected: Callback<String>, // room_id
    theme: Theme,
) -> impl IntoView {
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
            set_error_message.set(Some("Room ID cannot be empty".to_string()));
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

    let form_bg = theme.auth_form_bg;
    let input_bg = theme.auth_input_bg;
    let input_border = theme.auth_input_border;
    let input_text = theme.auth_input_text;
    let error_bg = theme.auth_error_bg;
    let error_border = theme.auth_error_border;
    let error_text = theme.auth_error_text;
    let button_color = theme.auth_button_room;

    view! {
        <div style="display: flex; flex-direction: column; align-items: center; justify-content: center; min-height: 100vh; padding: 20px;">
            <div style=format!("background: {}; padding: 40px; border-radius: 10px; max-width: 400px; width: 100%;", form_bg)>
                <h1 style="color: white; text-align: center; margin-bottom: 30px;">"Select Room"</h1>

                <form on:submit=on_submit style="display: flex; flex-direction: column; gap: 20px;">
                    <div style="display: flex; flex-direction: column; gap: 8px;">
                        <label style="color: #ccc;">"Room ID"</label>
                        <input
                            type="text"
                            value=move || room_id.get()
                            on:input=move |ev| set_room_id.set(event_target_value(&ev))
                            placeholder="Enter room ID (e.g., main-room)"
                            style=format!("padding: 12px; border-radius: 5px; border: 1px solid {}; background: {}; color: {}; font-size: 16px;", input_border, input_bg, input_text)
                        />
                    </div>

                    <Show when=move || error_message.get().is_some()>
                        <div style=format!("padding: 12px; background: {}; border: 1px solid {}; border-radius: 5px; color: {};", error_bg, error_border, error_text)>
                            {move || error_message.get().unwrap_or_default()}
                        </div>
                    </Show>

                    <button
                        type="submit"
                        style=format!("padding: 12px; background: {}; color: white; border: none; border-radius: 5px; font-size: 16px; cursor: pointer; font-weight: bold;", button_color)
                    >
                        "Join Room"
                    </button>
                </form>
            </div>
        </div>
    }
}
