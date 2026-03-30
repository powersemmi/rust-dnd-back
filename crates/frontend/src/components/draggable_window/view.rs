use super::view_model::DraggableWindowViewModel;
use crate::config::Theme;
use leptos::ev;
use leptos::prelude::*;
use leptos::web_sys::MouseEvent;

const WINDOW_TITLE_FONT_SIZE: &str = "clamp(1rem, 0.94rem + 0.22vw, 1.15rem)";
const WINDOW_CLOSE_FONT_SIZE: &str = "clamp(1rem, 0.95rem + 0.25vw, 1.2rem)";

/// A draggable, resizable floating window.
///
/// Drag by the header. Resize via the bottom-right corner handle.
/// Visibility is controlled by the `is_open` signal.
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
    let vm = DraggableWindowViewModel::new(
        initial_x,
        initial_y,
        initial_width,
        initial_height,
        min_width,
        min_height,
    );

    let on_header_mouse_down = move |ev: MouseEvent| {
        ev.prevent_default();
        vm.start_drag(ev.client_x(), ev.client_y());
    };

    let on_resize_mouse_down = move |ev: MouseEvent| {
        ev.prevent_default();
        ev.stop_propagation();
        vm.start_resize(ev.client_x(), ev.client_y());
    };

    let display = move || if is_open.get() { "flex" } else { "none" };

    // Global mouse move/up listeners handle drag/resize even when cursor leaves the window
    Effect::new(move || {
        use leptos::ev::MouseEvent as LeptosMouseEvent;

        let handle_mousemove = window_event_listener(ev::mousemove, move |ev: LeptosMouseEvent| {
            vm.update_drag(ev.client_x(), ev.client_y());
            vm.update_resize(ev.client_x(), ev.client_y());
        });

        let handle_mouseup = window_event_listener(ev::mouseup, move |_: LeptosMouseEvent| {
            vm.end_interaction();
        });

        on_cleanup(move || {
            drop(handle_mousemove);
            drop(handle_mouseup);
        });
    });

    view! {
        <div
            on:mousedown=move |_| {
                if let Some(cb) = on_focus {
                    cb.run(());
                }
            }
            on:mouseenter=move |_| {
                if let Some(cb) = on_focus {
                    cb.run(());
                }
            }
            style=move || {
                let opacity = if is_active.get() { "1" } else { "0.7" };
                format!(
                    "position: fixed; left: {}px; top: {}px; width: {}px; height: {}px; \
                     background: {}; border: 0.0625rem solid {}; border-radius: 0.75rem; \
                     box-shadow: 0 0.5rem 2rem rgba(0,0,0,0.38); z-index: 1001; \
                     display: {}; flex-direction: column; overflow: hidden; \
                     opacity: {}; transition: opacity 0.2s;",
                    vm.pos_x.get(),
                    vm.pos_y.get(),
                    vm.width.get(),
                    vm.height.get(),
                    theme.ui_bg_primary,
                    theme.ui_border,
                    display(),
                    opacity
                )
            }
        >
            // Header - drag handle
            <div
                on:mousedown=on_header_mouse_down
                style=format!(
                    "padding: 0.875rem 1rem; background: {}; border-bottom: 0.0625rem solid {}; \
                     cursor: move; display: flex; justify-content: space-between; \
                     align-items: center; user-select: none;",
                    theme.ui_bg_secondary, theme.ui_border
                )
            >
                <h3 style=format!(
                    "margin: 0; color: {}; font-size: {}; line-height: 1.2;",
                    theme.ui_text_primary, WINDOW_TITLE_FONT_SIZE
                )>
                    {move || title.get()}
                </h3>
                <button
                    on:click=move |_| is_open.set(false)
                    style=format!(
                        "background: {}; border: none; color: {}; padding: 0.375rem 0.75rem; \
                         border-radius: 0.5rem; cursor: pointer; font-size: {}; \
                         font-weight: bold; line-height: 1;",
                        theme.ui_button_danger, theme.ui_text_primary, WINDOW_CLOSE_FONT_SIZE
                    )
                >
                    "x"
                </button>
            </div>

            // Content area - provided by parent
            {children()}

            // Resize handle (bottom-right corner)
            <div
                on:mousedown=on_resize_mouse_down
                style=format!(
                    "position: absolute; bottom: 0; right: 0; width: 1.25rem; height: 1.25rem; \
                     cursor: nwse-resize; background: linear-gradient(135deg, transparent 50%, {} 50%);",
                    theme.ui_text_muted
                )
            />
        </div>
    }
}
