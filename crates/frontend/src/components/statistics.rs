use crate::components::draggable_window::DraggableWindow;
use crate::config::Theme;
use crate::i18n::i18n::{t, t_string, use_i18n};
use leptos::prelude::*;

#[derive(Clone, Debug)]
pub struct StateEvent {
    pub version: u64,
    pub event_type: String,
    pub description: String,
    pub timestamp: String,
}

#[component]
pub fn StatisticsWindow(
    #[prop(into)] is_open: RwSignal<bool>,
    #[prop(into)] events: RwSignal<Vec<StateEvent>>,
    theme: Theme,
) -> impl IntoView {
    let i18n = use_i18n();

    view! {
        <DraggableWindow
            is_open=is_open
            title=move || t_string!(i18n, statistics.title)
            initial_x=150
            initial_y=150
            initial_width=450
            initial_height=600
            min_width=350
            min_height=250
            theme=theme
        >
            <div
                style="
                    flex: 1;
                    overflow-y: auto;
                    padding: 15px;
                    display: flex;
                    flex-direction: column;
                    gap: 10px;
                "
            >
                <h4 style="margin: 0 0 10px 0; color: #aaa; font-size: 14px;">
                    {move || t!(i18n, statistics.event_log)}
                </h4>

                {move || {
                    if events.get().is_empty() {
                        view! {
                            <div style="color: #666; font-style: italic;">
                                {t!(i18n, statistics.no_events)}
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <For
                                each=move || events.get()
                                key=|event| event.version
                                let:event
                            >
                                <div
                                    style="
                                        padding: 10px;
                                        background: #2a2a2a;
                                        border-left: 3px solid #4a9eff;
                                        border-radius: 4px;
                                    "
                                >
                                    <div style="display: flex; justify-content: space-between; margin-bottom: 5px;">
                                        <span style="color: #4a9eff; font-weight: bold; font-size: 12px;">
                                            {format!("v{}", event.version)}
                                        </span>
                                        <span style="color: #888; font-size: 11px;">
                                            {event.timestamp.clone()}
                                        </span>
                                    </div>
                                    <div style="color: #ffaa00; font-size: 13px; margin-bottom: 3px;">
                                        {event.event_type.clone()}
                                    </div>
                                    <div style="color: #ccc; font-size: 12px;">
                                        {event.description.clone()}
                                    </div>
                                </div>
                            </For>
                        }.into_any()
                    }
                }}
            </div>
        </DraggableWindow>
    }
}
