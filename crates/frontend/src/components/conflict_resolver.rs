use super::websocket::{ConflictType, SyncConflict};
use crate::config::Theme;
use crate::i18n::i18n::{t_string, use_i18n};
use leptos::ev::MouseEvent;
use leptos::prelude::*;
use shared::events::voting::{VotingOption, VotingStartPayload, VotingType};

#[component]
pub fn ConflictResolver(
    conflict: RwSignal<Option<SyncConflict>>,
    username: ReadSignal<String>,
    on_create_voting: impl Fn(VotingStartPayload) + 'static + Copy + Send + Sync,
    on_submit_vote: impl Fn(String, Vec<String>) + 'static + Copy + Send + Sync,
    theme: Theme,
) -> impl IntoView {
    let i18n = use_i18n();
    let new_room_input = RwSignal::new(String::new());

    let on_move_to_new_room = move |_: MouseEvent| {
        let new_room = new_room_input.get();
        if !new_room.is_empty() {
            // Сохраняем текущий стейт в localStorage для новой комнаты
            if let Some(window) = web_sys::window() {
                if let Ok(Some(storage)) = window.local_storage() {
                    // Получаем текущую комнату из URL
                    if let Some(location) = window.location().search().ok() {
                        if let Some(old_room) = location
                            .split("room=")
                            .nth(1)
                            .map(|s| s.split('&').next().unwrap_or(""))
                        {
                            // Копируем стейт из старой комнаты в новую
                            if let Ok(Some(old_state_json)) =
                                storage.get_item(&format!("room_state:{}", old_room))
                            {
                                let _ = storage
                                    .set_item(&format!("room_state:{}", new_room), &old_state_json);
                            }
                        }
                    }
                }

                // Перенаправляем пользователя в новую комнату
                let _ = window.location().set_href(&format!("/?room={}", new_room));
            }
        }
    };

    let on_force_sync = move |_: MouseEvent| {
        // Создаём голосование с таймером 60 секунд
        let voting_id = format!("conflict_vote_{}", js_sys::Date::now() as u64);

        let payload = VotingStartPayload {
            voting_id: voting_id.clone(),
            question: t_string!(i18n, conflict.option_force_sync).to_string(),
            options: vec![
                VotingOption {
                    id: ".0".to_string(),
                    text: ".0".to_string(), // Will be displayed as "No" in UI
                },
                VotingOption {
                    id: ".1".to_string(),
                    text: ".1".to_string(), // Will be displayed as "Yes" in UI
                },
            ],
            voting_type: VotingType::SingleChoice,
            is_anonymous: false,
            timer_seconds: Some(60),
            default_option_id: Some(".0".to_string()), // Default to "No"
            creator: username.get(),
        };

        on_create_voting(payload);

        // Автоматически голосуем "Да" за инициатора
        on_submit_vote(voting_id, vec![".1".to_string()]);

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
            <div style="position: fixed; top: 0; left: 0; width: 100vw; height: 100vh; \
                background: rgba(0,0,0,0.8); z-index: 10000; \
                display: flex; align-items: center; justify-content: center;".to_string()>
                <div style=format!(
                    "background: {}; color: {}; padding: 1.875rem; border-radius: 0.75rem; \
                    max-width: 37.5rem; width: 90%; box-shadow: 0 0.25rem 1.25rem rgba(0,0,0,0.5);",
                    theme.ui_bg_primary, theme.ui_text_primary
                )>
                    <h2 style=format!("margin-top: 0; color: {};", theme.ui_button_danger)>
                        {move || t_string!(i18n, conflict.title)}
                    </h2>

                    <p style="margin: 1.25rem 0;">
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

                    <hr style=format!("border-color: {}; margin: 1.25rem 0;", theme.ui_border) />

                    <h3>{move || t_string!(i18n, conflict.options_title)}</h3>

                    <p style="margin-top: 0.9375rem;">
                        <strong>"1. "</strong>
                        {move || t_string!(i18n, conflict.option_move_room)}
                    </p>
                    <input
                        type="text"
                        placeholder={move || t_string!(i18n, conflict.new_room_placeholder)}
                        style=format!(
                            "width: calc(100% - 1.25rem); padding: 0.625rem; \
                            background: {}; color: {}; border: 0.0625rem solid {}; \
                            border-radius: 0.375rem; margin: 0.625rem 0;",
                            theme.ui_bg_secondary, theme.ui_text_primary, theme.ui_border
                        )
                        on:input=move |ev| {
                            new_room_input.set(event_target_value(&ev));
                        }
                        prop:value=move || new_room_input.get()
                    />
                    <button
                        style=format!(
                            "padding: 0.625rem 1.25rem; background: {}; color: {}; \
                            border: none; border-radius: 0.375rem; cursor: pointer; \
                            font-size: 0.875rem; margin-bottom: 0.9375rem;",
                            theme.ui_button_primary, theme.ui_text_primary
                        )
                        on:click=on_move_to_new_room
                    >
                        {move || t_string!(i18n, conflict.move_button)}
                    </button>

                    <p style="margin-top: 0.9375rem;">
                        <strong>"2. "</strong>
                        {move || t_string!(i18n, conflict.option_force_sync)}
                    </p>
                    <button
                        style=format!(
                            "padding: 0.625rem 1.25rem; background: #ff9800; color: {}; \
                            border: none; border-radius: 0.375rem; cursor: pointer; \
                            font-size: 0.875rem; margin-bottom: 0.9375rem;",
                            theme.ui_text_primary
                        )
                        on:click=on_force_sync
                    >
                        {move || t_string!(i18n, conflict.force_button)}
                    </button>

                    <p style="margin-top: 0.9375rem;">
                        <strong>"3. "</strong>
                        {move || t_string!(i18n, conflict.option_discard)}
                    </p>
                    <button
                        style=format!(
                            "padding: 0.625rem 1.25rem; background: {}; color: {}; \
                            border: none; border-radius: 0.375rem; cursor: pointer; \
                            font-size: 0.875rem;",
                            theme.ui_button_danger, theme.ui_text_primary
                        )
                        on:click=on_discard
                    >
                        {move || t_string!(i18n, conflict.discard_button)}
                    </button>
                </div>
            </div>
        </Show>
    }
}
