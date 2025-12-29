use crate::config::Theme;
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
    theme: Theme,
    children: Children,
) -> impl IntoView {
    let form_bg = theme.auth_form_bg;
    let button_color = theme.auth_button_room;

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

    view! {
        <div
            on:mousemove=on_global_mouse_move
            on:mouseup=on_global_mouse_up
            style=move || format!(
                "position: fixed; left: {}px; top: {}px; width: {}px; height: {}px; background: {}; border: 1px solid #444; border-radius: 8px; box-shadow: 0 4px 20px rgba(0,0,0,0.5); z-index: 1001; display: {}; flex-direction: column; overflow: hidden;",
                pos_x.get(),
                pos_y.get(),
                width.get(),
                height.get(),
                form_bg,
                display()
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
                        {move || title.get()}
                    </h3>
                    <button
                        on:click=move |_| is_open.set(false)
                        style=format!(
                            "background: {}; border: none; color: white; padding: 4px 10px; border-radius: 4px; cursor: pointer; font-size: 12px;",
                            button_color
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
    }
}
