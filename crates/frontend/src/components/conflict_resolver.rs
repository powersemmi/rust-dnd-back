use super::websocket::{ConflictType, SyncConflict};
use crate::config::Theme;
use crate::i18n::i18n::{t_string, use_i18n};
use leptos::ev::MouseEvent;
use leptos::prelude::*;

#[component]
pub fn ConflictResolver(conflict: RwSignal<Option<SyncConflict>>, theme: Theme) -> impl IntoView {
    let i18n = use_i18n();
    let new_room_input = RwSignal::new(String::new());

    let on_move_to_new_room = move |_: MouseEvent| {
        let new_room = new_room_input.get();
        if !new_room.is_empty() {
            // Перенаправляем пользователя в новую комнату
            if let Some(window) = web_sys::window() {
                let _ = window.location().set_href(&format!("/?room={}", new_room));
            }
        }
    };

    let on_force_sync = move |_: MouseEvent| {
        // Отправляем SyncSnapshot с нашей версией всем пользователям
        // TODO: реализовать отправку через websocket
        conflict.set(None);
    };

    let on_discard = move |_: MouseEvent| {
        // Перезагружаем страницу, чтобы получить актуальный стейт
        if let Some(window) = web_sys::window() {
            let _ = window.location().reload();
        }
    };

    view! {
        <Show when=move || conflict.get().is_some()>
            <div style=format!(
                "position: fixed; top: 0; left: 0; width: 100vw; height: 100vh; \
                background: rgba(0,0,0,0.8); z-index: 10000; \
                display: flex; align-items: center; justify-content: center;"
            )>
                <div style=format!(
                    "background: {}; color: {}; padding: 30px; border-radius: 12px; \
                    max-width: 600px; width: 90%; box-shadow: 0 4px 20px rgba(0,0,0,0.5);",
                    theme.auth_form_bg, theme.auth_input_text
                )>
                    <h2 style="margin-top: 0; color: #ff6b6b;">
                        {move || t_string!(i18n, conflict.title)}
                    </h2>

                    <p style="margin: 20px 0;">
                        {move || {
                            conflict.get().as_ref().map(|c| {
                                match c.conflict_type {
                                    ConflictType::SplitBrain => t_string!(i18n, conflict.split_brain),
                                    ConflictType::Fork => t_string!(i18n, conflict.fork),
                                    ConflictType::UnsyncedLocal => t_string!(i18n, conflict.unsynced_local),
                                }
                            }).unwrap_or_default()
                        }}
                    </p>

                    <p>
                        <strong>{move || t_string!(i18n, conflict.local_version)}</strong>
                        {move || conflict.get().as_ref().map(|c| format!(": v{}", c.local_version)).unwrap_or_default()}
                    </p>
                    <p>
                        <strong>{move || t_string!(i18n, conflict.remote_version)}</strong>
                        {move || conflict.get().as_ref().map(|c| format!(": v{}", c.remote_version)).unwrap_or_default()}
                    </p>

                    <hr style="border-color: #444; margin: 20px 0;" />

                    <h3>{move || t_string!(i18n, conflict.options_title)}</h3>

                    <p style="margin-top: 15px;">
                        <strong>"1. "</strong>
                        {move || t_string!(i18n, conflict.option_move_room)}
                    </p>
                    <input
                        type="text"
                        placeholder={move || t_string!(i18n, conflict.new_room_placeholder)}
                        style=format!(
                            "width: calc(100% - 20px); padding: 10px; \
                            background: {}; color: {}; border: 1px solid #555; \
                            border-radius: 6px; margin: 10px 0;",
                            theme.auth_input_bg, theme.auth_input_text
                        )
                        on:input=move |ev| {
                            new_room_input.set(event_target_value(&ev));
                        }
                        prop:value=move || new_room_input.get()
                    />
                    <button
                        style=format!(
                            "padding: 10px 20px; background: {}; color: white; \
                            border: none; border-radius: 6px; cursor: pointer; \
                            font-size: 14px; margin-bottom: 15px;",
                            theme.auth_button_room
                        )
                        on:click=on_move_to_new_room
                    >
                        {move || t_string!(i18n, conflict.move_button)}
                    </button>

                    <p style="margin-top: 15px;">
                        <strong>"2. "</strong>
                        {move || t_string!(i18n, conflict.option_force_sync)}
                    </p>
                    <button
                        style="padding: 10px 20px; background: #ff9800; color: white; \
                            border: none; border-radius: 6px; cursor: pointer; \
                            font-size: 14px; margin-bottom: 15px;"
                        on:click=on_force_sync
                    >
                        {move || t_string!(i18n, conflict.force_button)}
                    </button>

                    <p style="margin-top: 15px;">
                        <strong>"3. "</strong>
                        {move || t_string!(i18n, conflict.option_discard)}
                    </p>
                    <button
                        style="padding: 10px 20px; background: #f44336; color: white; \
                            border: none; border-radius: 6px; cursor: pointer; \
                            font-size: 14px;"
                        on:click=on_discard
                    >
                        {move || t_string!(i18n, conflict.discard_button)}
                    </button>
                </div>
            </div>
        </Show>
    }
}
