use super::super::chat::ChatWindow;
use super::super::conflict_resolver::ConflictResolver;
use super::super::language_selector::LanguageSelector;
use super::super::login::LoginForm;
use super::super::notes::NotesWindow;
use super::super::register::RegisterForm;
use super::super::room_selector::RoomSelector;
use super::super::scene_board::SceneBoard;
use super::super::scenes::ScenesWindow;
use super::super::settings::{
    Settings, load_inactive_scene_contents_visibility, load_workspace_hint_visibility,
    save_inactive_scene_contents_visibility, save_workspace_hint_visibility,
};
use super::super::side_menu::SideMenu;
use super::super::statistics::StatisticsWindow;
use super::super::tokens::TokensWindow;
use super::super::websocket::{
    ConflictResolutionHandle, CursorSignals, FileTransferState, StoredNoteBucket,
    StoredTokenLibraryItem, SyncConflict, WsSender, delete_state, load_notes,
};
use super::model::ActiveWindow;
use super::navigation::create_room_selected_callback;
use super::view_model::AppViewModel;
use super::{AppState, create_login_success_callback, create_navigation_callbacks};
use crate::components::statistics::model::StateEvent;
use crate::components::voting::VotingWindow;
use crate::config;
use crate::i18n::i18n::{I18nContextProvider, Locale};
use crate::utils::{auth, token_refresh};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos::wasm_bindgen::JsCast;
use shared::events::{ChatMessagePayload, NotePayload, Scene};
use std::collections::HashMap;

#[component]
pub fn App() -> impl IntoView {
    let initial_locale = web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|s| s.get_item("locale").ok().flatten())
        .map(|s| if s == "ru" { Locale::ru } else { Locale::en })
        .unwrap_or(Locale::en);

    let cfg = StoredValue::new(config::Config::default());
    let theme = StoredValue::new(cfg.get_value().theme.clone());
    let back_url = cfg.get_value().api.back_url;
    let api_path = cfg.get_value().api.api_path;

    let initial_state = if auth::load_and_validate_token().is_some() {
        AppState::RoomSelection
    } else {
        AppState::Login
    };

    // Root ViewModel - owns all window/notification state
    let vm = AppViewModel::new(initial_state);

    let (jwt_token, set_jwt_token) = signal(String::new());
    let (username, set_username) = signal(String::new());
    let (room_id, set_room_id) = signal(String::new());

    let (cursors, set_cursors) = signal(HashMap::<String, CursorSignals>::new());
    let messages = RwSignal::new(Vec::<ChatMessagePayload>::new());
    let public_notes = RwSignal::new(Vec::<NotePayload>::new());
    let private_notes = RwSignal::new(Vec::<NotePayload>::new());
    let direct_notes = RwSignal::new(Vec::<NotePayload>::new());
    let direct_note_recipients = RwSignal::new(Vec::<String>::new());
    let direct_note_recipients_cache_updated_at_ms = RwSignal::new(Option::<f64>::None);
    let direct_note_recipients_request_id = RwSignal::new(Option::<String>::None);
    let state_events = RwSignal::new(Vec::<StateEvent>::new());
    let scenes = RwSignal::new(Vec::<Scene>::new());
    let active_scene_id = RwSignal::new(Option::<String>::None);
    let token_library_items = RwSignal::new(Vec::<StoredTokenLibraryItem>::new());
    let dragging_library_token_id = RwSignal::new(Option::<String>::None);
    let show_workspace_hint = RwSignal::new(load_workspace_hint_visibility().unwrap_or(true));
    let show_inactive_scene_contents =
        RwSignal::new(load_inactive_scene_contents_visibility().unwrap_or(false));
    let voting_results =
        RwSignal::new(HashMap::<String, shared::events::voting::VotingResultPayload>::new());
    let conflict_signal = RwSignal::new(Option::<SyncConflict>::None);
    let votings = RwSignal::new(HashMap::<String, super::super::voting::VotingState>::new());
    let voted_in = RwSignal::new(std::collections::HashSet::<String>::new());
    let selected_options_map = RwSignal::new(std::collections::HashMap::<
        String,
        std::collections::HashSet<String>,
    >::new());
    let (ws_sender, set_ws_sender) = signal::<Option<WsSender>>(None);
    let file_transfer = FileTransferState::new();
    let conflict_resolution_handle = ConflictResolutionHandle::new();

    let clear_room_local_state = {
        let handle = conflict_resolution_handle.clone();
        Callback::new(move |_| {
            let current_room = room_id.get_untracked();
            if current_room.is_empty() {
                return;
            }
            let handle = handle.clone();
            spawn_local(async move {
                match delete_state(&current_room).await {
                    Ok(()) => leptos::logging::log!(
                        "Cleared local IndexedDB room state for '{}'",
                        current_room
                    ),
                    Err(error) => leptos::logging::log!(
                        "Failed to clear local IndexedDB room state for '{}': {}",
                        current_room,
                        error
                    ),
                }
                handle.invoke();
            });
        })
    };

    // Load persisted token on initial RoomSelection state
    if initial_state == AppState::RoomSelection {
        if let Some(token) = auth::load_and_validate_token() {
            set_jwt_token.set(token);
            token_refresh::start_token_refresh(back_url, api_path);
            if let Some(user) = auth::load_username() {
                set_username.set(user);
            }
        } else {
            vm.app_state.set(AppState::Login);
        }
    }

    // Navigation callbacks
    let on_login_success = StoredValue::new(create_login_success_callback(
        set_jwt_token,
        set_username,
        vm.app_state.write_only(),
        back_url,
        api_path,
    ));

    let (on_registered, on_switch_to_register, on_switch_to_login) =
        create_navigation_callbacks(vm.app_state.write_only());
    let on_registered = StoredValue::new(on_registered);
    let on_switch_to_register = StoredValue::new(on_switch_to_register);
    let on_switch_to_login = StoredValue::new(on_switch_to_login);

    let on_room_selected = StoredValue::new(create_room_selected_callback(
        super::navigation::RoomSelectedCallbackArgs {
            set_room_id,
            set_app_state: vm.app_state.write_only(),
            jwt_token,
            username,
            file_transfer: file_transfer.clone(),
            set_ws_sender,
            set_cursors,
            messages,
            public_notes,
            direct_notes,
            direct_note_recipients,
            direct_note_recipients_cache_updated_at_ms,
            direct_note_recipients_request_id,
            state_events,
            scenes,
            active_scene_id,
            conflict_signal,
            votings,
            voting_results,
            has_statistics_notification: vm.has_statistics_notification,
            notification_count: vm.notification_count,
            has_chat_notification: vm.has_chat_notification,
            chat_notification_count: vm.chat_notification_count,
            cfg,
            conflict_resolution_handle: conflict_resolution_handle.clone(),
        },
    ));

    {
        let file_transfer = file_transfer.clone();
        Effect::new(move |_| {
            if vm.app_state.get() != AppState::Connected {
                return;
            }
            let current_room = room_id.get();
            let current_username = username.get();
            if current_room.is_empty() || current_username.is_empty() {
                return;
            }
            file_transfer.reconcile_scene_files(
                current_room,
                current_username,
                ws_sender.get(),
                scenes.get(),
            );
        });
    }

    Effect::new(move |_| {
        let _ = room_id.get();
        token_library_items.set(Vec::new());
        dragging_library_token_id.set(None);
        public_notes.set(Vec::new());
        private_notes.set(Vec::new());
        direct_notes.set(Vec::new());
        direct_note_recipients.set(Vec::new());
        direct_note_recipients_cache_updated_at_ms.set(None);
        direct_note_recipients_request_id.set(None);
    });

    Effect::new(move |_| {
        if vm.app_state.get() != AppState::Connected {
            return;
        }

        let current_room = room_id.get();
        let current_user = username.get();
        if current_room.is_empty() || current_user.is_empty() {
            return;
        }

        spawn_local(async move {
            if let Ok(notes) =
                load_notes(&current_room, &current_user, StoredNoteBucket::Private).await
            {
                private_notes.set(notes);
            }
            if let Ok(notes) =
                load_notes(&current_room, &current_user, StoredNoteBucket::Direct).await
            {
                direct_notes.set(notes);
            }
        });
    });

    Effect::new(move |_| {
        save_workspace_hint_visibility(show_workspace_hint.get());
    });

    Effect::new(move |_| {
        save_inactive_scene_contents_visibility(show_inactive_scene_contents.get());
    });

    // Keyboard shortcut handler - delegates to ViewModel
    let on_keydown = move |ev: web_sys::KeyboardEvent| {
        if vm.app_state.get() != AppState::Connected {
            return;
        }
        // Skip when focus is in an input/textarea
        if let Some(target) = ev.target()
            && let Some(element) = target.dyn_ref::<web_sys::HtmlElement>()
        {
            let tag = element.tag_name().to_lowercase();
            if tag == "input" || tag == "textarea" {
                return;
            }
        }
        vm.handle_hotkey(&ev.code());
    };

    // Auto-focus the container when switching to Connected state
    Effect::new(move || {
        if vm.app_state.get() == AppState::Connected
            && let Some(window) = web_sys::window()
            && let Some(document) = window.document()
            && let Some(element) = document.get_element_by_id("main-app-container")
        {
            let _ = element
                .dyn_ref::<web_sys::HtmlElement>()
                .map(|el| el.focus());
        }
    });

    let bg_color = theme.get_value().background_color;

    view! {
        <I18nContextProvider>
            <div
                id="main-app-container"
                tabindex="0"
                style=format!(
                    "width: 100vw; height: 100vh; background: {}; overflow: hidden; outline: none;",
                    bg_color
                )
                on:keydown=on_keydown
            >
                <Show when=move || vm.app_state.get() != AppState::Connected>
                    <LanguageSelector initial_locale=initial_locale theme=theme.get_value() />
                </Show>

                {move || match vm.app_state.get() {
                    AppState::Login => view! {
                        <LoginForm
                            on_login_success=Callback::new(move |token| on_login_success.get_value()(token))
                            on_switch_to_register=Callback::new(move |_| on_switch_to_register.get_value()(()))
                            back_url=back_url
                            api_path=api_path
                            theme=theme.get_value()
                        />
                    }.into_any(),
                    AppState::Register => view! {
                        <RegisterForm
                            on_registered=Callback::new(move |_| on_registered.get_value()(()))
                            on_switch_to_login=Callback::new(move |_| on_switch_to_login.get_value()(()))
                            back_url=back_url
                            api_path=api_path
                            theme=theme.get_value()
                        />
                    }.into_any(),
                    AppState::RoomSelection => view! {
                        <RoomSelector
                            on_room_selected=Callback::new(move |room| on_room_selected.get_value()(room))
                            theme=theme.get_value()
                        />
                    }.into_any(),
                    AppState::Connected => view! {
                        <div style="width: 100%; height: 100%; position: relative;">
                            <SceneBoard
                                room_id=room_id
                                scenes=scenes
                                active_scene_id=active_scene_id
                                public_notes=public_notes
                                private_notes=private_notes
                                direct_notes=direct_notes
                                show_workspace_hint=show_workspace_hint
                                show_inactive_scene_contents=show_inactive_scene_contents
                                token_library_items=token_library_items
                                dragging_library_token_id=dragging_library_token_id
                                cursors=cursors
                                set_cursors=set_cursors
                                file_transfer=file_transfer.clone()
                                ws_sender=ws_sender
                                username=username
                                config=cfg.get_value()
                                theme=theme.get_value()
                            />

                            <h3 style="color: #aaa; position: absolute; top: 10px; right: 10px; z-index: 100;">
                                "Connected as: " {move || username.get()} " | Room: " {move || room_id.get()}
                            </h3>

                            <SideMenu
                                is_open=vm.is_menu_open
                                on_chat_open=Callback::new(move |_| vm.open_chat())
                                on_notes_open=Callback::new(move |_| vm.open_notes())
                                on_scenes_open=Callback::new(move |_| vm.open_scenes())
                                on_tokens_open=Callback::new(move |_| vm.open_tokens())
                                on_settings_open=Callback::new(move |_| vm.open_settings())
                                on_statistics_open=Callback::new(move |_| vm.open_statistics())
                                on_voting_open=Callback::new(move |_| vm.open_voting())
                                has_statistics_notification=vm.has_statistics_notification.read_only()
                                notification_count=vm.notification_count.read_only()
                                has_chat_notification=vm.has_chat_notification.read_only()
                                chat_notification_count=vm.chat_notification_count.read_only()
                                theme=theme.get_value()
                            />

                            <ChatWindow
                                is_open=vm.is_chat_open
                                messages=messages
                                file_transfer=file_transfer.clone()
                                ws_sender=ws_sender
                                username=username
                                is_active=Signal::derive(move || vm.active_window.get() == ActiveWindow::Chat)
                                on_focus=Callback::new(move |_| vm.active_window.set(ActiveWindow::Chat))
                                theme=theme.get_value()
                            />

                            <NotesWindow
                                is_open=vm.is_notes_open
                                room_id=room_id
                                username=username
                                ws_sender=ws_sender
                                public_notes=public_notes
                                private_notes=private_notes
                                direct_notes=direct_notes
                                direct_note_recipients=direct_note_recipients
                                direct_note_recipients_cache_updated_at_ms=direct_note_recipients_cache_updated_at_ms
                                direct_note_recipients_request_id=direct_note_recipients_request_id
                                is_active=Signal::derive(move || vm.active_window.get() == ActiveWindow::Notes)
                                on_focus=Callback::new(move |_| vm.active_window.set(ActiveWindow::Notes))
                                theme=theme.get_value()
                            />

                            <Settings
                                is_open=vm.is_settings_open
                                show_workspace_hint=show_workspace_hint
                                show_inactive_scene_contents=show_inactive_scene_contents
                                on_clear_room_local_state=clear_room_local_state
                                current_room=room_id
                                theme=theme.get_value()
                            />

                            <ScenesWindow
                                is_open=vm.is_scenes_open
                                scenes=scenes
                                active_scene_id=active_scene_id
                                file_transfer=file_transfer.clone()
                                ws_sender=ws_sender
                                username=username
                                is_active=Signal::derive(move || vm.active_window.get() == ActiveWindow::Scenes)
                                on_focus=Callback::new(move |_| vm.active_window.set(ActiveWindow::Scenes))
                                theme=theme.get_value()
                            />

                            <TokensWindow
                                is_open=vm.is_tokens_open
                                room_id=room_id
                                items=token_library_items
                                file_transfer=file_transfer.clone()
                                ws_sender=ws_sender
                                username=username
                                on_start_drag=Callback::new(move |item: StoredTokenLibraryItem| {
                                    dragging_library_token_id.set(Some(item.id));
                                })
                                is_active=Signal::derive(move || vm.active_window.get() == ActiveWindow::Tokens)
                                on_focus=Callback::new(move |_| vm.active_window.set(ActiveWindow::Tokens))
                                theme=theme.get_value()
                            />

                            <StatisticsWindow
                                is_open=vm.is_statistics_open
                                events=state_events
                                voting_results=voting_results
                                is_active=Signal::derive(move || vm.active_window.get() == ActiveWindow::Statistics)
                                on_focus=Callback::new(move |_| vm.active_window.set(ActiveWindow::Statistics))
                                theme=theme.get_value()
                            />

                            <ConflictResolver
                                conflict=conflict_signal
                                username=username
                                on_create_voting=move |mut payload| {
                                    payload.creator = username.get();
                                    if let Some(sender) = ws_sender.get() {
                                        let request_id = format!("voting_{}", payload.voting_id);
                                        let presence_req = shared::events::PresenceRequestPayload {
                                            request_id,
                                            requester: username.get(),
                                        };
                                        let _ = sender.try_send_event(
                                            shared::events::ClientEvent::PresenceRequest(presence_req),
                                        );
                                        let _ = sender.try_send_event(
                                            shared::events::ClientEvent::VotingStart(payload),
                                        );
                                    }
                                }
                                on_submit_vote={
                                    let submit_vote_fn = move |voting_id: String, selected_option_ids: Vec<String>| {
                                        let voting_id_clone = voting_id.clone();
                                        let payload = shared::events::VotingCastPayload {
                                            voting_id,
                                            user: username.get(),
                                            selected_option_ids,
                                        };
                                        if let Some(sender) = ws_sender.get() {
                                            let _ = sender.try_send_event(
                                                shared::events::ClientEvent::VotingCast(payload),
                                            );
                                        }
                                        voted_in.update(|set| { set.insert(voting_id_clone); });
                                    };
                                    submit_vote_fn
                                }
                                on_change_room=move |new_room: String| {
                                    on_room_selected.get_value()(new_room)
                                }
                                current_room=room_id
                                votings=votings
                                ws_sender=ws_sender
                                voted_in=voted_in
                                selected_options_map=selected_options_map
                                theme=theme.get_value()
                                on_start_conflict_resolution={
                                    let handle = conflict_resolution_handle.clone();
                                    move || { handle.invoke(); }
                                }
                            />

                            <VotingWindow
                                show_voting_window=vm.is_voting_open
                                votings=votings
                                voted_in=voted_in
                                username=username
                                ws_sender=ws_sender
                                is_active=Signal::derive(move || vm.active_window.get() == ActiveWindow::Voting)
                                on_focus=Callback::new(move |_| vm.active_window.set(ActiveWindow::Voting))
                                on_create_voting=move |mut payload| {
                                    payload.creator = username.get();
                                    if let Some(sender) = ws_sender.get() {
                                        let request_id = format!("voting_{}", payload.voting_id);
                                        let presence_req = shared::events::PresenceRequestPayload {
                                            request_id,
                                            requester: username.get(),
                                        };
                                        let _ = sender.try_send_event(
                                            shared::events::ClientEvent::PresenceRequest(presence_req),
                                        );
                                        let _ = sender.try_send_event(
                                            shared::events::ClientEvent::VotingStart(payload),
                                        );
                                    }
                                }
                                theme=theme.get_value()
                            />
                        </div>
                    }.into_any(),
                }}
            </div>
        </I18nContextProvider>
    }
}
