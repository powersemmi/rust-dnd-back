use super::model::token_rect;
use crate::config::Theme;
use leptos::prelude::*;
use shared::events::Token;
use std::collections::HashMap;

#[component]
pub fn SceneTokenLayer(
    tokens: Vec<Token>,
    cell_size: f64,
    dragging_token_id: Option<String>,
    file_urls: HashMap<String, String>,
    theme: Theme,
) -> impl IntoView {
    view! {
        <>
            {tokens.into_iter().map(|token| {
                let (left, top, width, height) = token_rect(
                    0.0,
                    0.0,
                    cell_size,
                    token.x,
                    token.y,
                    token.width_cells,
                    token.height_cells,
                );
                let image_url = file_urls.get(&token.image.hash).cloned();
                let is_dragging = dragging_token_id.as_deref() == Some(token.id.as_str());
                let border = if is_dragging { theme.ui_button_primary } else { theme.ui_border };
                let shadow = if is_dragging {
                    "0 18px 36px rgba(0,0,0,0.32), 0 0 0 2px rgba(255,255,255,0.08)"
                } else {
                    "0 10px 24px rgba(0,0,0,0.22), 0 0 0 1px rgba(255,255,255,0.06)"
                };
                let transition = if is_dragging {
                    "none"
                } else {
                    "left 180ms ease, top 180ms ease, box-shadow 180ms ease, border-color 180ms ease"
                };
                let label_font = (cell_size * 0.18).clamp(11.0, 15.0);

                view! {
                    <div style=format!(
                        "position: absolute; left: {:.2}px; top: {:.2}px; width: {:.2}px; height: {:.2}px; \
                         border: 2px solid {}; border-radius: {:.2}px; overflow: hidden; box-shadow: {}; \
                         background: rgba(15,23,42,0.62); z-index: {}; pointer-events: none; transition: {};",
                        left, top, width, height, border, (cell_size * 0.18).clamp(8.0, 16.0), shadow,
                        if is_dragging { 4 } else { 3 }, transition
                    )>
                        {match image_url {
                            Some(url) => view! {
                                <img
                                    src=url
                                    alt=token.name.clone()
                                    style="width: 100%; height: 100%; object-fit: cover; display: block;"
                                />
                            }.into_any(),
                            None => view! {
                                <div style="width: 100%; height: 100%; display: flex; align-items: center; justify-content: center; background: linear-gradient(135deg, rgba(255,255,255,0.08), rgba(255,255,255,0.02));">
                                    <span style=format!("font-size: {:.2}px; color: {}; font-weight: 700;", label_font, theme.ui_text_secondary)>
                                        {token.name.chars().next().unwrap_or('?').to_string()}
                                    </span>
                                </div>
                            }.into_any(),
                        }}
                        <div style="position: absolute; left: 0.35rem; right: 0.35rem; bottom: 0.35rem; padding: 0.22rem 0.35rem; background: rgba(0,0,0,0.48); border-radius: 0.45rem; backdrop-filter: blur(6px);">
                            <div style=format!("font-size: {:.2}px; font-weight: 700; color: {}; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;", label_font, theme.ui_text_primary)>
                                {token.name.clone()}
                            </div>
                        </div>
                    </div>
                }
            }).collect_view()}
        </>
    }
}
