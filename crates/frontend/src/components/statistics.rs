use crate::components::draggable_window::DraggableWindow;
use crate::components::tab_bar::{TabBar, TabItem};
use crate::config::Theme;
use crate::i18n::i18n::{t, t_string, use_i18n};
use leptos::prelude::*;
use shared::events::voting::VotingResultPayload;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct StateEvent {
    pub version: u64,
    pub event_type: String,
    pub description: String,
    pub timestamp: String,
}

#[derive(Clone, Copy, PartialEq)]
enum StatisticsTab {
    VotingResults,
    EventLog,
}

#[component]
pub fn StatisticsWindow(
    #[prop(into)] is_open: RwSignal<bool>,
    #[prop(into)] events: RwSignal<Vec<StateEvent>>,
    #[prop(into)] voting_results: RwSignal<HashMap<String, VotingResultPayload>>,
    #[prop(into, optional)] is_active: Signal<bool>,
    #[prop(optional)] on_focus: Option<Callback<()>>,
    theme: Theme,
) -> impl IntoView {
    let i18n = use_i18n();
    let active_tab = RwSignal::new(StatisticsTab::VotingResults);

    let tabs = move || {
        vec![
            TabItem::new(
                StatisticsTab::VotingResults,
                t_string!(i18n, statistics.tab_voting_results),
            ),
            TabItem::new(
                StatisticsTab::EventLog,
                t_string!(i18n, statistics.tab_event_log),
            ),
        ]
    };

    view! {
        <DraggableWindow
            is_open=is_open
            title=move || t_string!(i18n, statistics.title)
            initial_x=150
            initial_y=150
            initial_width=450
            initial_height=600
            min_width=350
            is_active=is_active
            on_focus=on_focus.unwrap_or_else(|| Callback::new(|_| {}))
            min_height=250
            theme=theme.clone()
        >
            <div style="display: flex; flex-direction: column; height: 100%;">
                <TabBar tabs=tabs active_tab=active_tab theme=theme.clone() />

                // Tab content
                <div
                    style="
                        flex: 1;
                        overflow-y: auto;
                        padding: 0.25rem 1rem 3rem 1rem;
                    "
                >
                    {move || match active_tab.get() {
                        StatisticsTab::VotingResults => view! {
                            <div>
                                {move || {
                                    let results = voting_results.get();
                                    if results.is_empty() {
                                        view! {
                                            <div style=format!("color: {}; font-style: italic;", theme.ui_text_muted)>
                                                {t!(i18n, statistics.no_voting_results)}
                                            </div>
                                        }.into_any()
                                    } else {
                                        view! {
                                            <For
                                                each=move || {
                                                    voting_results.get().into_iter().collect::<Vec<_>>()
                                                }
                                                key=|(voting_id, _)| voting_id.clone()
                                                children=move |(_, result)| {
                                        view! {
                                            <div
                                                style=format!(
                                                    "padding: 0.75rem; background: {}; border-left: 0.1875rem solid {}; border-radius: 0.25rem; margin-bottom: 0.625rem;",
                                                    theme.ui_bg_primary, theme.ui_success
                                                )
                                            >
                                                <div style=format!("color: {}; font-weight: bold; margin-bottom: 0.5rem; font-size: 1rem;", theme.ui_text_primary)>
                                                    {result.question.clone()}
                                                </div>
                                                <div style=format!("color: {}; font-size: 0.8125rem; margin-bottom: 0.75rem;", theme.ui_text_secondary)>
                                                    {format!("Total: {} participants, {} voted", result.total_participants, result.total_voted)}
                                                </div>
                                                <div style="display: flex; flex-direction: column; gap: 0.5rem;">
                                                    <For
                                                        each=move || result.results.clone()
                                                        key=|opt_result| opt_result.option_id.clone()
                                                        children=move |opt_result| {
                                                            let percentage = if result.total_voted > 0 {
                                                                (opt_result.count as f32 / result.total_voted as f32 * 100.0) as u32
                                                            } else {
                                                                0
                                                            };

                                                            // Находим текст опции по ID
                                                            let option_text = result.options.iter()
                                                                .find(|opt| opt.id == opt_result.option_id)
                                                                .map(|opt| opt.text.clone())
                                                                .unwrap_or_else(|| opt_result.option_id.clone());

                                                            view! {
                                                                <div style=format!("background: {}; padding: 0.5rem; border-radius: 0.25rem;", theme.ui_bg_secondary)>
                                                                    <div style="display: flex; justify-content: space-between; margin-bottom: 0.375rem;">
                                                                        <span style=format!("color: {}; font-size: 0.875rem; font-weight: 500;", theme.ui_text_primary)>{option_text}</span>
                                                                        <span style=format!("color: {}; font-size: 0.8125rem; font-weight: bold;", theme.ui_button_primary)>
                                                                            {format!("{} votes ({}%)", opt_result.count, percentage)}
                                                                        </span>
                                                                    </div>
                                                                    <div style=format!("width: 100%; height: 0.5rem; background: {}; border-radius: 0.25rem; overflow: hidden;", theme.ui_border)>
                                                                        <div style=format!("height: 100%; background: {}; width: {}%;", theme.ui_button_primary, percentage) />
                                                                    </div>
                                                                </div>
                                                            }
                                                        }
                                                    />
                                                </div>
                                            </div>
                                        }
                                    }
                                />
                            }.into_any()
                        }
                    }}
                            </div>
                        }.into_any(),
                        StatisticsTab::EventLog => view! {
                            <div>
                                {move || {
                                    if events.get().is_empty() {
                                        view! {
                                            <div style=format!("color: {}; font-style: italic;", theme.ui_text_muted)>
                                                {t!(i18n, statistics.no_events)}
                                            </div>
                                        }.into_any()
                                    } else {
                                        view! {
                                            <For
                                                each=move || events.get()
                                                key=|event| event.version
                                                children=move |event| {
                                                    view! {
                                                        <div
                                                            style=format!(
                                                                "padding: 0.625rem; background: {}; border-left: 0.1875rem solid {}; border-radius: 0.25rem; margin-bottom: 0.625rem;",
                                                                theme.ui_bg_primary, theme.ui_button_primary
                                                            )
                                                        >
                                                            <div style="display: flex; justify-content: space-between; margin-bottom: 0.3125rem;">
                                                                <span style=format!("color: {}; font-weight: bold; font-size: 0.75rem;", theme.ui_button_primary)>
                                                                    {format!("v{}", event.version)}
                                                                </span>
                                                                <span style=format!("color: {}; font-size: 0.6875rem;", theme.ui_text_muted)>
                                                                    {event.timestamp.clone()}
                                                                </span>
                                                            </div>
                                                            <div style="color: #ffaa00; font-size: 0.8125rem; margin-bottom: 0.1875rem;">
                                                                {event.event_type.clone()}
                                                            </div>
                                                            <div style=format!("color: {}; font-size: 0.75rem;", theme.ui_text_secondary)>
                                                                {event.description.clone()}
                                                            </div>
                                                        </div>
                                                    }
                                                }
                                            />
                                        }.into_any()
                                    }
                                }}
                            </div>
                        }.into_any(),
                    }}
                </div>
            </div>
        </DraggableWindow>
    }
}
