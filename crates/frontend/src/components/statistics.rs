use crate::config::Theme;
use crate::i18n::i18n::{t, use_i18n};
use leptos::prelude::*;
use leptos::web_sys::MouseEvent;

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

    let form_bg = theme.auth_form_bg;
    let button_color = theme.auth_button_room;

    let (pos_x, set_pos_x) = signal(150);
    let (pos_y, set_pos_y) = signal(150);
    let (width, set_width) = signal(450);
    let (height, set_height) = signal(600);
    let (is_dragging, set_is_dragging) = signal(false);
    let (drag_start_x, set_drag_start_x) = signal(0);
    let (drag_start_y, set_drag_start_y) = signal(0);
    let (window_start_x, set_window_start_x) = signal(0);
    let (window_start_y, set_window_start_y) = signal(0);
    let (is_resizing, set_is_resizing) = signal(false);
    let (resize_start_x, set_resize_start_x) = signal(0);
    let (resize_start_y, set_resize_start_y) = signal(0);
    let (size_start_w, set_size_start_w) = signal(0);
    let (size_start_h, set_size_start_h) = signal(0);

    let on_header_mouse_down = move |ev: MouseEvent| {
        ev.prevent_default();
        set_is_dragging.set(true);
        set_drag_start_x.set(ev.client_x());
        set_drag_start_y.set(ev.client_y());
        set_window_start_x.set(pos_x.get());
        set_window_start_y.set(pos_y.get());
    };

    let on_resize_mouse_down = move |ev: MouseEvent| {
        ev.prevent_default();
        ev.stop_propagation();
        set_is_resizing.set(true);
        set_resize_start_x.set(ev.client_x());
        set_resize_start_y.set(ev.client_y());
        set_size_start_w.set(width.get());
        set_size_start_h.set(height.get());
    };

    let on_global_mouse_move = move |ev: MouseEvent| {
        if is_dragging.get() {
            let dx = ev.client_x() - drag_start_x.get();
            let dy = ev.client_y() - drag_start_y.get();
            set_pos_x.set(window_start_x.get() + dx);
            set_pos_y.set(window_start_y.get() + dy);
        } else if is_resizing.get() {
            let dx = ev.client_x() - resize_start_x.get();
            let dy = ev.client_y() - resize_start_y.get();
            set_width.set((size_start_w.get() + dx).max(350));
            set_height.set((size_start_h.get() + dy).max(250));
        }
    };

    let on_global_mouse_up = move |_: MouseEvent| {
        set_is_dragging.set(false);
        set_is_resizing.set(false);
    };

    view! {
        <Show when=move || is_open.get()>
            <div
                on:mousemove=on_global_mouse_move
                on:mouseup=on_global_mouse_up
                style=move || format!(
                    "position: fixed; left: {}px; top: {}px; width: {}px; height: {}px; background: {}; border: 1px solid #444; border-radius: 8px; box-shadow: 0 4px 20px rgba(0,0,0,0.5); z-index: 1001; display: flex; flex-direction: column; overflow: hidden;",
                    pos_x.get(),
                    pos_y.get(),
                    width.get(),
                    height.get(),
                    form_bg
                )
            >
                // Header
                <div
                    on:mousedown=on_header_mouse_down
                    style="
                        padding: 12px 15px;
                        background: #2a2a2a;
                        border-bottom: 1px solid #444;
                        cursor: move;
                        display: flex;
                        justify-content: space-between;
                        align-items: center;
                        user-select: none;
                    "
                >
                    <h3 style="margin: 0; color: white; font-size: 16px;">
                        {move || t!(i18n, statistics.title)}
                    </h3>
                    <button
                        on:click=move |_| is_open.set(false)
                        style=format!(
                            "background: {}; border: none; color: white; padding: 4px 10px; border-radius: 4px; cursor: pointer; font-size: 12px;",
                            button_color
                        )
                    >
                        {move || t!(i18n, statistics.close)}
                    </button>
                </div>

                // Event list area
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

                // Resize handle
                <div
                    on:mousedown=on_resize_mouse_down
                    style="
                        position: absolute;
                        bottom: 0;
                        right: 0;
                        width: 20px;
                        height: 20px;
                        cursor: nwse-resize;
                        background: linear-gradient(135deg, transparent 50%, #666 50%);
                    "
                />
            </div>
        </Show>
    }
}
