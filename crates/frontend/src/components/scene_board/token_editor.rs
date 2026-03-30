use super::model::{MAX_TOKEN_SIZE_CELLS, MIN_TOKEN_SIZE_CELLS};
use crate::config::Theme;
use crate::i18n::i18n::{t, t_string, use_i18n};
use leptos::ev::MouseEvent;
use leptos::portal::Portal;
use leptos::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SceneTokenEditorDraft {
    pub scene_id: String,
    pub token_id: String,
    pub name: String,
    pub width_cells: String,
    pub height_cells: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SceneTokenEditorValue {
    pub scene_id: String,
    pub token_id: String,
    pub name: String,
    pub width_cells: u16,
    pub height_cells: u16,
}

fn validate_editor_draft(
    draft: &SceneTokenEditorDraft,
    empty_name_error: &str,
    invalid_dimensions_error: &str,
) -> Result<SceneTokenEditorValue, String> {
    let name = draft.name.trim();
    if name.is_empty() {
        return Err(empty_name_error.to_string());
    }

    let Ok(width_cells) = draft.width_cells.parse::<u16>() else {
        return Err(invalid_dimensions_error.to_string());
    };
    let Ok(height_cells) = draft.height_cells.parse::<u16>() else {
        return Err(invalid_dimensions_error.to_string());
    };

    if !(MIN_TOKEN_SIZE_CELLS..=MAX_TOKEN_SIZE_CELLS).contains(&width_cells)
        || !(MIN_TOKEN_SIZE_CELLS..=MAX_TOKEN_SIZE_CELLS).contains(&height_cells)
    {
        return Err(invalid_dimensions_error.to_string());
    }

    Ok(SceneTokenEditorValue {
        scene_id: draft.scene_id.clone(),
        token_id: draft.token_id.clone(),
        name: name.to_string(),
        width_cells,
        height_cells,
    })
}

#[component]
pub fn SceneTokenEditor(
    #[prop(into)] draft: RwSignal<Option<SceneTokenEditorDraft>>,
    on_save: Callback<SceneTokenEditorValue>,
    on_close: Callback<()>,
    theme: Theme,
) -> impl IntoView {
    let i18n = use_i18n();
    let error = RwSignal::new(None::<String>);

    Effect::new(move |_| {
        if draft.get().is_none() {
            error.set(None);
        }
    });

    view! {
        <Show when=move || draft.get().is_some()>
            <Portal>
                <div
                    on:mousedown=move |_| on_close.run(())
                    style="position: fixed; top: 0; left: 0; width: 100vw; height: 100vh; z-index: 2200; \
                           background: rgba(2, 6, 23, 0.58); backdrop-filter: blur(8px); display: flex; \
                           align-items: center; justify-content: center; padding: 1.25rem;"
                >
                    <div
                        on:mousedown=move |event: MouseEvent| event.stop_propagation()
                        on:click=move |event| event.stop_propagation()
                        style=format!(
                            "width: min(30rem, calc(100vw - 2.5rem)); padding: 1.35rem; border-radius: 1rem; \
                             border: 0.0625rem solid {}; background: linear-gradient(180deg, {}, {}); \
                             box-shadow: 0 1.5rem 4rem rgba(0,0,0,0.38); display: flex; flex-direction: column; gap: 1rem;",
                            theme.ui_border, theme.ui_bg_primary, theme.ui_bg_secondary
                        )
                    >
                        <div style="display: flex; justify-content: space-between; gap: 1rem; align-items: flex-start;">
                            <div style="min-width: 0;">
                                <div style=format!("color: {}; font-size: clamp(1rem, 0.95rem + 0.2vw, 1.12rem); font-weight: 800; line-height: 1.15;", theme.ui_text_primary)>
                                    {move || t!(i18n, tokens.editor_title)}
                                </div>
                                <div style=format!("color: {}; font-size: clamp(0.84rem, 0.81rem + 0.12vw, 0.92rem); margin-top: 0.35rem; line-height: 1.45;", theme.ui_text_secondary)>
                                    {move || t!(i18n, tokens.editor_hint)}
                                </div>
                            </div>
                            <button
                                type="button"
                                on:mousedown=move |event: MouseEvent| {
                                    event.prevent_default();
                                    event.stop_propagation();
                                    on_close.run(());
                                }
                                style=format!(
                                    "background: {}; border: none; color: {}; padding: 0.35rem 0.7rem; border-radius: 0.5rem; cursor: pointer; font-size: 0.9rem; font-weight: 700;",
                                    theme.ui_button_danger, theme.ui_text_primary
                                )
                            >
                                "x"
                            </button>
                        </div>

                        <div style="display: flex; flex-direction: column; gap: 0.8rem;">
                            <label style=format!("color: {}; display: flex; flex-direction: column; gap: 0.35rem;", theme.ui_text_secondary)>
                                <span>{move || t!(i18n, tokens.name_label)}</span>
                                <input
                                    type="text"
                                    prop:value=move || draft.get().map(|draft| draft.name).unwrap_or_default()
                                    on:input=move |event| {
                                        let value = event_target_value(&event);
                                        draft.update(|draft| {
                                            if let Some(draft) = draft.as_mut() {
                                                draft.name = value.clone();
                                            }
                                        });
                                    }
                                    style=format!(
                                        "padding: 0.75rem; background: {}; color: {}; border: 0.0625rem solid {}; border-radius: 0.6rem; font-size: 0.95rem;",
                                        theme.ui_bg_secondary, theme.ui_text_primary, theme.ui_border
                                    )
                                />
                            </label>

                            <div style="display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 0.8rem;">
                                <label style=format!("color: {}; display: flex; flex-direction: column; gap: 0.35rem;", theme.ui_text_secondary)>
                                    <span>{move || t!(i18n, tokens.width_label)}</span>
                                    <input
                                        type="number"
                                        min=MIN_TOKEN_SIZE_CELLS.to_string()
                                        max=MAX_TOKEN_SIZE_CELLS.to_string()
                                        prop:value=move || draft.get().map(|draft| draft.width_cells).unwrap_or_default()
                                        on:input=move |event| {
                                            let value = event_target_value(&event);
                                            draft.update(|draft| {
                                                if let Some(draft) = draft.as_mut() {
                                                    draft.width_cells = value.clone();
                                                }
                                            });
                                        }
                                        style=format!(
                                            "padding: 0.75rem; background: {}; color: {}; border: 0.0625rem solid {}; border-radius: 0.6rem; font-size: 0.95rem;",
                                            theme.ui_bg_secondary, theme.ui_text_primary, theme.ui_border
                                        )
                                    />
                                </label>

                                <label style=format!("color: {}; display: flex; flex-direction: column; gap: 0.35rem;", theme.ui_text_secondary)>
                                    <span>{move || t!(i18n, tokens.height_label)}</span>
                                    <input
                                        type="number"
                                        min=MIN_TOKEN_SIZE_CELLS.to_string()
                                        max=MAX_TOKEN_SIZE_CELLS.to_string()
                                        prop:value=move || draft.get().map(|draft| draft.height_cells).unwrap_or_default()
                                        on:input=move |event| {
                                            let value = event_target_value(&event);
                                            draft.update(|draft| {
                                                if let Some(draft) = draft.as_mut() {
                                                    draft.height_cells = value.clone();
                                                }
                                            });
                                        }
                                        style=format!(
                                            "padding: 0.75rem; background: {}; color: {}; border: 0.0625rem solid {}; border-radius: 0.6rem; font-size: 0.95rem;",
                                            theme.ui_bg_secondary, theme.ui_text_primary, theme.ui_border
                                        )
                                    />
                                </label>
                            </div>
                        </div>

                        {move || {
                            error.get().map(|error| {
                                view! {
                                    <div style=format!(
                                        "padding: 0.75rem 0.85rem; border-radius: 0.75rem; background: rgba(220, 38, 38, 0.16); \
                                         color: #fecaca; font-size: 0.92rem; line-height: 1.45;",
                                    )>
                                        {error}
                                    </div>
                                }.into_any()
                            }).unwrap_or_else(|| ().into_any())
                        }}

                        <div style=format!("display: flex; justify-content: flex-end; gap: 0.75rem; padding-top: 0.2rem; border-top: 0.0625rem solid {};", theme.ui_border)>
                            <button
                                type="button"
                                on:mousedown=move |event: MouseEvent| {
                                    event.prevent_default();
                                    event.stop_propagation();
                                    on_close.run(());
                                }
                                style=format!(
                                    "padding: 0.75rem 1rem; background: {}; color: {}; border: none; border-radius: 0.6rem; cursor: pointer; font-size: 0.92rem;",
                                    theme.ui_bg_secondary, theme.ui_text_primary
                                )
                            >
                                {move || t!(i18n, tokens.cancel_button)}
                            </button>
                            <button
                                type="button"
                                on:mousedown=move |event: MouseEvent| {
                                    event.prevent_default();
                                    event.stop_propagation();
                                    let Some(current_draft) = draft.get_untracked() else {
                                        return;
                                    };
                                    match validate_editor_draft(
                                        &current_draft,
                                        &t_string!(i18n, tokens.error_name_required),
                                        &t_string!(i18n, tokens.error_dimensions_invalid),
                                    ) {
                                        Ok(value) => {
                                            error.set(None);
                                            on_save.run(value);
                                        }
                                        Err(message) => error.set(Some(message)),
                                    }
                                }
                                style=format!(
                                    "padding: 0.75rem 1rem; background: {}; color: {}; border: none; border-radius: 0.6rem; cursor: pointer; font-size: 0.92rem; font-weight: 700;",
                                    theme.ui_button_primary, theme.ui_text_primary
                                )
                            >
                                {move || t!(i18n, tokens.save_button)}
                            </button>
                        </div>
                    </div>
                </div>
            </Portal>
        </Show>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_editor_draft_accepts_valid_values() {
        let draft = SceneTokenEditorDraft {
            scene_id: "scene-1".to_string(),
            token_id: "token-1".to_string(),
            name: "Ogre".to_string(),
            width_cells: "2".to_string(),
            height_cells: "3".to_string(),
        };

        let value = validate_editor_draft(&draft, "name", "dimensions").unwrap();
        assert_eq!(value.name, "Ogre");
        assert_eq!(value.width_cells, 2);
        assert_eq!(value.height_cells, 3);
    }

    #[test]
    fn validate_editor_draft_rejects_invalid_dimensions() {
        let draft = SceneTokenEditorDraft {
            scene_id: "scene-1".to_string(),
            token_id: "token-1".to_string(),
            name: "Ogre".to_string(),
            width_cells: "0".to_string(),
            height_cells: "3".to_string(),
        };

        assert_eq!(
            validate_editor_draft(&draft, "name", "dimensions").unwrap_err(),
            "dimensions"
        );
    }
}
