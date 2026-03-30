use super::voting::{VotingActive, VotingState};
use super::websocket::{ConflictType, SyncConflict, WsSender, delete_state, move_state};
use crate::config::Theme;
use crate::i18n::i18n::{t_string, use_i18n};
use leptos::prelude::*;
use leptos::task::spawn_local;
use shared::events::voting::{VotingOption, VotingStartPayload, VotingType};
use std::collections::{HashMap, HashSet};

#[component]
pub fn ConflictResolver(
    conflict: RwSignal<Option<SyncConflict>>,
    username: ReadSignal<String>,
    on_create_voting: impl Fn(VotingStartPayload) + 'static + Clone + Send + Sync,
    on_submit_vote: impl Fn(String, Vec<String>) + 'static + Clone + Send + Sync,
    on_change_room: impl Fn(String) + 'static + Clone + Send + Sync,
    current_room: ReadSignal<String>,
    votings: RwSignal<HashMap<String, VotingState>>,
    ws_sender: ReadSignal<Option<WsSender>>,
    voted_in: RwSignal<HashSet<String>>,
    selected_options_map: RwSignal<HashMap<String, HashSet<String>>>,
    theme: Theme,
    on_start_conflict_resolution: impl Fn() + 'static + Clone + Send + Sync,
) -> impl IntoView {
    let i18n = use_i18n();
    let new_room_input = RwSignal::new(String::new());
    let theme_stored = StoredValue::new(theme);

    // Проверяем, есть ли активное голосование для разрешения конфликта
    let active_conflict_voting_id = Memo::new(move |_| {
        votings.with(|map| {
            map.iter()
                .find(|(id, state)| {
                    id.starts_with("conflict_vote_") && matches!(state, VotingState::Active { .. })
                })
                .map(|(id, _)| id.clone())
        })
    });

    // Store клоны для использования в обработчиках
    let on_change_room_stored = StoredValue::new(on_change_room);
    let on_create_voting_stored = StoredValue::new(on_create_voting);
    let on_submit_vote_stored = StoredValue::new(on_submit_vote);
    let on_start_conflict_resolution_stored = StoredValue::new(on_start_conflict_resolution);

    view! {
        <Show when=move || conflict.get().is_some()>
            <div style="position: fixed; top: 0; left: 0; width: 100vw; height: 100vh; \
                background: rgba(0,0,0,0.8); z-index: 10000; \
                display: flex; align-items: center; justify-content: center;".to_string()>
                <div style=format!(
                    "background: {}; color: {}; padding: 1.875rem; border-radius: 0.75rem; \
                    max-width: 37.5rem; width: 90%; box-shadow: 0 0.25rem 1.25rem rgba(0,0,0,0.5); \
                    max-height: 90vh; overflow-y: auto;",
                    theme_stored.get_value().ui_bg_primary, theme_stored.get_value().ui_text_primary
                )>
                    <h2 style=format!("margin-top: 0; color: {};", theme_stored.get_value().ui_button_danger)>
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

                    <hr style=format!("border-color: {}; margin: 1.25rem 0;", theme_stored.get_value().ui_border) />

                    // Если есть активное голосование - показываем его, иначе показываем опции
                    {move || {
                        if let Some(voting_id) = active_conflict_voting_id.get() {
                            // Показываем компонент голосования
                            view! {
                                <div>
                                    <h3 style="margin-top: 0;">{move || t_string!(i18n, conflict.voting_in_progress)}</h3>
                                    <VotingActive
                                        voting_id=voting_id
                                        voting=votings
                                        username=username
                                        ws_sender=ws_sender
                                        voted_in=voted_in
                                        selected_options_map=selected_options_map
                                        theme=theme_stored.get_value()
                                    />
                                </div>
                            }.into_any()
                        } else {
                            // Показываем опции конфликта
                            view! {
                                <div>
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
                                            theme_stored.get_value().ui_bg_secondary, theme_stored.get_value().ui_text_primary, theme_stored.get_value().ui_border
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
                                            theme_stored.get_value().ui_button_primary, theme_stored.get_value().ui_text_primary
                                        )
                                        on:click=move |_| {
                                            let new_room = new_room_input.get();
                                            if !new_room.is_empty() {
                                                let old_room = current_room.get();
                                                spawn_local(async move {
                                                    if let Err(error) = move_state(&old_room, &new_room).await {
                                                        leptos::logging::log!(
                                                            "Failed to move IndexedDB room state from '{}' to '{}': {}",
                                                            old_room,
                                                            new_room,
                                                            error
                                                        );
                                                    }

                                                    conflict.set(None);
                                                    on_change_room_stored.with_value(|f| f.clone()(new_room));
                                                });
                                            }
                                        }
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
                                            theme_stored.get_value().ui_text_primary
                                        )
                                        on:click=move |_| {
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

                                            on_create_voting_stored.with_value(|f| f.clone()(payload));

                                            // Автоматически голосуем "Да" за инициатора
                                            on_submit_vote_stored.with_value(|f| f.clone()(voting_id, vec![".1".to_string()]));

                                            // НЕ закрываем окно конфликта - пусть пользователи проголосуют
                                            // Окно останется открытым, пока конфликт не будет разрешён
                                        }
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
                                            theme_stored.get_value().ui_button_danger, theme_stored.get_value().ui_text_primary
                                        )
                                        on:click=move |_| {
                                            let room = current_room.get();
                                            spawn_local(async move {
                                                match delete_state(&room).await {
                                                    Ok(()) => leptos::logging::log!(
                                                        "🗑️ Cleared IndexedDB state for room: {}",
                                                        room
                                                    ),
                                                    Err(error) => leptos::logging::log!(
                                                        "Failed to clear IndexedDB state for room '{}': {}",
                                                        room,
                                                        error
                                                    ),
                                                }

                                                leptos::logging::log!("🔄 Discarded local changes, starting conflict resolution...");

                                                conflict.set(None);
                                                on_start_conflict_resolution_stored.with_value(|f| f.clone()());
                                            });
                                        }
                                    >
                                        {move || t_string!(i18n, conflict.discard_button)}
                                    </button>
                                </div>
                            }.into_any()
                        }
                    }}
                </div>
            </div>
        </Show>
    }
}
