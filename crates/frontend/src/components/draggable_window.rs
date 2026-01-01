use crate::config::Theme;
use leptos::logging::log;
use leptos::prelude::*;
use leptos::web_sys::MouseEvent;

#[component]
pub fn DraggableWindow(
    #[prop(into)] is_open: RwSignal<bool>,
    #[prop(into)] title: Signal<String>,
    #[prop(default = 100)] initial_x: i32,
    #[prop(default = 100)] initial_y: i32,
    #[prop(default = 400)] initial_width: i32,
    #[prop(default = 500)] initial_height: i32,
    #[prop(default = 300)] min_width: i32,
    #[prop(default = 200)] min_height: i32,
    #[prop(into, optional)] is_active: Signal<bool>,
    #[prop(optional)] on_focus: Option<Callback<()>>,
    theme: Theme,
    children: Children,
) -> impl IntoView {
    let (pos_x, set_pos_x) = signal(initial_x);
    let (pos_y, set_pos_y) = signal(initial_y);
    let (width, set_width) = signal(initial_width);
    let (height, set_height) = signal(initial_height);
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
            set_width.set((size_start_w.get() + dx).max(min_width));
            set_height.set((size_start_h.get() + dy).max(min_height));
        }
    };

    let on_global_mouse_up = move |_: MouseEvent| {
        set_is_dragging.set(false);
        set_is_resizing.set(false);
    };

    let display = move || if is_open.get() { "flex" } else { "none" };

    log!("DraggableWindow:{}", theme.ui_bg_primary);

    view! {
        <div
            on:mousemove=on_global_mouse_move
            on:mouseup=on_global_mouse_up
            on:mousedown=move |_| {
                if let Some(callback) = on_focus {
                    callback.run(());
                }
            }
            on:mouseenter=move |_| {
                if let Some(callback) = on_focus {
                    callback.run(());
                }
            }
            style=move || {
                let opacity = if is_active.get() { "1" } else { "0.7" };
                format!(
                    "position: fixed; left: {}px; top: {}px; width: {}px; height: {}px; background: {}; border: 0.0625rem solid {}; border-radius: 0.5rem; box-shadow: 0 0.25rem 1.25rem rgba(0,0,0,0.5); z-index: 1001; display: {}; flex-direction: column; overflow: hidden; opacity: {}; transition: opacity 0.2s;",
                    pos_x.get(),
                    pos_y.get(),
                    width.get(),
                    height.get(),
                    theme.ui_bg_primary,
                    theme.ui_border,
                    display(),
                    opacity
                )
            }
        >
                // Header
                <div
                    on:mousedown=on_header_mouse_down
                    style=format!(
                        "padding: 0.75rem 0.9375rem; background: {}; border-bottom: 0.0625rem solid {}; cursor: move; display: flex; justify-content: space-between; align-items: center; user-select: none;",
                        theme.ui_bg_secondary, theme.ui_border
                    )
                >
                    <h3 style=format!("margin: 0; color: {}; font-size: 1rem;", theme.ui_text_primary)>
                        {move || title.get()}
                    </h3>
                    <button
                        on:click=move |_| is_open.set(false)
                        style=format!(
                            "background: {}; border: none; color: {}; padding: 0.25rem 0.625rem; border-radius: 0.25rem; cursor: pointer; font-size: 1.125rem; font-weight: bold; line-height: 1;",
                            theme.ui_button_danger, theme.ui_text_primary
                        )
                    >
                        "×"
                    </button>
                </div>

                // Content area
                {children()}


                // Resize handle
                <div
                    on:mousedown=on_resize_mouse_down
                    style=format!(
                        "position: absolute; bottom: 0; right: 0; width: 1.25rem; height: 1.25rem; cursor: nwse-resize; background: linear-gradient(135deg, transparent 50%, {} 50%);",
                        theme.ui_text_muted
                    )
                />
            </div>
    }
}
