use leptos::prelude::*;

/// Компонент одного курсора
#[component]
pub fn Cursor(
    username: String,
    #[prop(into)] x: Signal<i32>,
    #[prop(into)] y: Signal<i32>,
    #[prop(into)] visible: Signal<bool>,
    is_me: bool,
    theme: crate::config::Theme,
) -> impl IntoView {
    let color = if is_me {
        theme.my_cursor_color
    } else {
        theme.other_cursor_color
    };

    let style = move || {
        let opacity = if visible.get() { "1" } else { "0" };
        format!(
            "position: absolute; left: {}px; top: {}px; pointer-events: none; transition: {}, opacity 0.3s ease-out; z-index: 100; opacity: {};",
            x.get(),
            y.get(),
            &theme.cursor_transition,
            opacity
        )
    };

    view! {
        <div style=style>
            // Стрелочка курсора
            <svg width={theme.cursor_size.to_string()} height={theme.cursor_size.to_string()} viewBox="0 0 24 24" fill={color}>
                <path d="M7 2l12 11.2-5.8.5 3.3 7.3-2.2.9-3.2-7.4-4.4 4z"/>
            </svg>
            // Никнейм
            <span style="background: rgba(0,0,0,0.7); color: white; padding: 0.125rem 0.3125rem; border-radius: 0.25rem; font-size: 0.75rem;">
                {username}
            </span>
        </div>
    }
}
