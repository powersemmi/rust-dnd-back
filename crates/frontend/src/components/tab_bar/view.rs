use super::model::TabItem;
use crate::config::Theme;
use leptos::prelude::*;

/// A generic tab navigation bar.
///
/// `tabs` - a function returning the list of tab items (called reactively).
/// `active_tab` - the currently selected tab value.
/// `on_close` - optional callback fired when a closable tab's X is clicked.
#[component]
pub fn TabBar<T, F>(
    tabs: F,
    active_tab: RwSignal<T>,
    theme: Theme,
    #[prop(optional, into)] on_close: Option<Callback<T>>,
) -> impl IntoView
where
    T: Clone + PartialEq + Send + Sync + 'static,
    F: Fn() -> Vec<TabItem<T>> + Send + 'static,
{
    view! {
        <div style=format!(
            "display: flex; gap: 0.625rem; margin-bottom: 0.75rem; \
             border-bottom: 0.125rem solid {}; padding: 0.5rem 0.75rem 0.5rem 0.75rem;",
            theme.ui_bg_primary
        )>
            <For
                each=tabs
                key=|item| item.label.clone()
                children=move |item| {
                    let value = item.value.clone();
                    let value_for_style = value.clone();
                    let value_for_click = value.clone();
                    let label = item.label.clone();
                    let closable = item.closable;
                    view! {
                        <button
                            style=move || {
                                let base = "padding: 0.5rem 1rem; border: none; border-radius: 0.25rem; \
                                            cursor: pointer; font-size: 0.875rem; font-weight: 500; \
                                            display: flex; align-items: center; gap: 0.5rem;";
                                if active_tab.get() == value_for_style.clone() {
                                    format!("{} background: {}; color: {};", base, theme.ui_button_primary, theme.ui_text_primary)
                                } else {
                                    format!("{} background: {}; color: {};", base, theme.ui_bg_secondary, theme.ui_text_secondary)
                                }
                            }
                            on:click=move |_| active_tab.set(value_for_click.clone())
                        >
                            <span>{label.clone()}</span>
                            {if closable {
                                let value_for_close = value.clone();
                                view! {
                                    <span
                                        style="cursor: pointer; margin-left: 0.25rem; font-weight: bold;"
                                        on:click=move |e| {
                                            e.stop_propagation();
                                            if let Some(ref cb) = on_close {
                                                cb.run(value_for_close.clone());
                                            }
                                        }
                                    >
                                        "x"
                                    </span>
                                }.into_any()
                            } else {
                                ().into_any()
                            }}
                        </button>
                    }
                }
            />
        </div>
    }
}
