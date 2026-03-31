use super::model::BoardTool;
use crate::config::Theme;
use leptos::ev::MouseEvent;
use leptos::prelude::*;

/// Icon labels for each tool button.
const RULER_ICON: &str = "📏";
const POINTER_ICON: &str = "🖱";

#[component]
pub fn BoardToolbar(active_tool: RwSignal<BoardTool>, theme: Theme) -> impl IntoView {
    let stop = move |ev: MouseEvent| {
        ev.stop_propagation();
        ev.prevent_default();
    };

    let btn_style = move |tool: BoardTool| {
        let is_active = active_tool.get() == tool;
        let bg = if is_active {
            theme.ui_button_primary
        } else {
            theme.ui_bg_secondary
        };
        format!(
            "display: flex; align-items: center; justify-content: center; \
             width: 2.2rem; height: 2.2rem; border: 1px solid {}; border-radius: 0.5rem; \
             background: {}; color: {}; cursor: pointer; font-size: 1.1rem; \
             transition: background 150ms ease, border-color 150ms ease; line-height: 1;",
            theme.ui_border, bg, theme.ui_text_primary,
        )
    };

    let toggle = move |tool: BoardTool| {
        active_tool.update(|current| {
            if *current == tool {
                *current = BoardTool::None;
            } else {
                *current = tool;
            }
        });
    };

    view! {
        <div
            on:mousedown=stop.clone()
            on:click=stop
            style=format!(
                "position: absolute; right: 1rem; bottom: 5rem; display: flex; \
                 flex-direction: column; gap: 0.45rem; z-index: 6;"
            )
        >
            // Ruler tool
            <button
                title="Ruler (DnD distance)"
                on:mousedown=move |ev: MouseEvent| {
                    ev.stop_propagation();
                    ev.prevent_default();
                    toggle(BoardTool::Ruler);
                }
                style=move || btn_style(BoardTool::Ruler)
            >
                {RULER_ICON}
            </button>

            // Pointer tool
            <button
                title="Pointer (visible to others)"
                on:mousedown=move |ev: MouseEvent| {
                    ev.stop_propagation();
                    ev.prevent_default();
                    toggle(BoardTool::Pointer);
                }
                style=move || btn_style(BoardTool::Pointer)
            >
                {POINTER_ICON}
            </button>
        </div>
    }
}

/// Ruler overlay: shows a line and distance label between two world points.
/// Rendered in screen coordinates inside the board viewport.
#[component]
pub fn RulerOverlay(
    /// Ruler start in screen coordinates.
    start_screen_x: f64,
    start_screen_y: f64,
    /// Ruler end in screen coordinates.
    end_screen_x: f64,
    end_screen_y: f64,
    /// Distance in cells.
    distance_cells: f64,
    /// Distance in feet.
    distance_feet: f64,
) -> impl IntoView {
    // Mid-point for label
    let mid_x = (start_screen_x + end_screen_x) / 2.0;
    let mid_y = (start_screen_y + end_screen_y) / 2.0;

    let label = if distance_feet >= 5.0 {
        format!("{:.0} ft ({:.1} sq)", distance_feet, distance_cells)
    } else {
        format!("{:.1} sq", distance_cells)
    };

    view! {
        // SVG line
        <svg
            style="position: absolute; inset: 0; pointer-events: none; overflow: visible; z-index: 10;"
            width="100%"
            height="100%"
        >
            // Line
            <line
                x1=format!("{:.2}", start_screen_x)
                y1=format!("{:.2}", start_screen_y)
                x2=format!("{:.2}", end_screen_x)
                y2=format!("{:.2}", end_screen_y)
                stroke="#facc15"
                stroke-width="2"
                stroke-dasharray="6 3"
            />
            // Start dot
            <circle
                cx=format!("{:.2}", start_screen_x)
                cy=format!("{:.2}", start_screen_y)
                r="5"
                fill="#facc15"
            />
            // End dot
            <circle
                cx=format!("{:.2}", end_screen_x)
                cy=format!("{:.2}", end_screen_y)
                r="5"
                fill="#facc15"
            />
        </svg>
        // Distance label
        <div style=format!(
            "position: absolute; left: {:.2}px; top: {:.2}px; \
             transform: translate(-50%, -130%); \
             background: rgba(0,0,0,0.75); color: #facc15; \
             font-size: 0.78rem; font-weight: 700; padding: 0.2rem 0.5rem; \
             border-radius: 0.4rem; pointer-events: none; z-index: 11; white-space: nowrap; \
             box-shadow: 0 2px 8px rgba(0,0,0,0.4);",
            mid_x, mid_y
        )>
            {label}
        </div>
    }
}

/// Pointer trail overlay: renders a colored trail path for a remote user's pointer tool.
#[component]
pub fn PointerTrailOverlay(
    username: String,
    /// Trail points in screen coordinates (oldest first, newest last).
    points: Vec<(f64, f64)>,
    /// Whether the pointer is currently active.
    active: bool,
    theme: Theme,
) -> impl IntoView {
    if points.is_empty() {
        return ().into_any();
    }

    let tip = points.last().copied().unwrap_or((0.0, 0.0));

    // Build SVG polyline points string
    let polyline_pts = points
        .iter()
        .map(|(x, y)| format!("{x:.2},{y:.2}"))
        .collect::<Vec<_>>()
        .join(" ");

    let cursor_color = theme.other_cursor_color;

    view! {
        <svg
            style="position: absolute; inset: 0; pointer-events: none; overflow: visible; z-index: 9;"
            width="100%"
            height="100%"
        >
            // Trail
            <polyline
                points=polyline_pts
                fill="none"
                stroke=cursor_color
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
                opacity=if active { "0.7" } else { "0.3" }
            />
            // Tip circle
            {if active {
                view! {
                    <circle
                        cx=format!("{:.2}", tip.0)
                        cy=format!("{:.2}", tip.1)
                        r="6"
                        fill=cursor_color
                        opacity="0.9"
                    />
                }.into_any()
            } else {
                ().into_any()
            }}
        </svg>
        // Username label near tip
        {if active {
            view! {
                <div style=format!(
                    "position: absolute; left: {:.2}px; top: {:.2}px; \
                     transform: translate(10px, -50%); \
                     background: rgba(0,0,0,0.6); color: {}; \
                     font-size: 0.7rem; padding: 0.1rem 0.35rem; \
                     border-radius: 0.3rem; pointer-events: none; z-index: 10;",
                    tip.0, tip.1, cursor_color
                )>
                    {username.clone()}
                </div>
            }.into_any()
        } else {
            ().into_any()
        }}
    }.into_any()
}

/// Attention ping animation: pulsing ring at a screen position.
#[component]
pub fn AttentionPingAnimation(
    screen_x: f64,
    screen_y: f64,
    username: String,
    theme: Theme,
) -> impl IntoView {
    view! {
        <div
            style=format!(
                "position: absolute; left: {:.2}px; top: {:.2}px; \
                 transform: translate(-50%, -50%); pointer-events: none; z-index: 12;",
                screen_x, screen_y
            )
        >
            // Pulsing rings via CSS animation
            <div style=format!(
                "width: 40px; height: 40px; border-radius: 50%; \
                 border: 3px solid {}; animation: ping 1s ease-out 3 forwards;",
                theme.ui_button_primary
            ) />
            // Username label
            <div style=format!(
                "position: absolute; top: -1.4rem; left: 50%; transform: translateX(-50%); \
                 background: {}; color: {}; font-size: 0.65rem; font-weight: 700; \
                 padding: 0.1rem 0.3rem; border-radius: 0.3rem; white-space: nowrap;",
                theme.ui_button_primary, theme.ui_text_primary
            )>
                {username}
            </div>
        </div>
    }
}
