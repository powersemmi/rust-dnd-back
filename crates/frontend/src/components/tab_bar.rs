use crate::config::Theme;
use leptos::prelude::*;

#[derive(Clone)]
pub struct TabItem<T> {
    pub value: T,
    pub label: String,
    pub closable: bool,
}

impl<T> TabItem<T> {
    pub fn new(value: T, label: impl Into<String>) -> Self {
        Self {
            value,
            label: label.into(),
            closable: false,
        }
    }

    pub fn closable(mut self) -> Self {
        self.closable = true;
        self
    }
}

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
            "display: flex; gap: 0.625rem; margin-bottom: 0.75rem; border-bottom: 0.125rem solid {}; padding: 0.5rem 0.75rem 0.5rem 0.75rem;",
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
                                let base = "padding: 0.5rem 1rem; border: none; border-radius: 0.25rem; cursor: pointer; font-size: 0.875rem; font-weight: 500; display: flex; align-items: center; gap: 0.5rem;";
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
                                let on_close_cb = on_close.clone();
                                view! {
                                    <span
                                        style="cursor: pointer; margin-left: 0.25rem; font-weight: bold;"
                                        on:click=move |e| {
                                            e.stop_propagation();
                                            if let Some(ref cb) = on_close_cb {
                                                cb.run(value_for_close.clone());
                                            }
                                        }
                                    >
                                        "×"
                                    </span>
                                }.into_any()
                            } else {
                                view! { <></> }.into_any()
                            }}
                        </button>
                    }
                }
            />
        </div>
    }
}
