use leptos::prelude::*;

#[component]
pub fn SideMenu(
    #[prop(into)] is_open: RwSignal<bool>,
    on_chat_open: Callback<()>,
) -> impl IntoView {
    let toggle_menu = move |_| {
        is_open.update(|open| *open = !*open);
    };

    view! {
        <div>
            // Кнопка открытия меню
            <button
                on:click=toggle_menu
                style="
                    position: fixed;
                    top: 20px;
                    left: 20px;
                    z-index: 1000;
                    padding: 10px 15px;
                    background: #333;
                    color: white;
                    border: none;
                    border-radius: 5px;
                    cursor: pointer;
                    font-size: 18px;
                "
            >
                "☰"
            </button>

            // Боковое меню
            <div
                style=move || format!(
                    "
                    position: fixed;
                    top: 0;
                    left: {};
                    width: 250px;
                    height: 100vh;
                    background: #2a2a2a;
                    box-shadow: 2px 0 10px rgba(0,0,0,0.3);
                    transition: left 0.3s ease;
                    z-index: 999;
                    padding: 70px 20px 20px 20px;
                    ",
                    if is_open.get() { "0" } else { "-250px" }
                )
            >
                <div style="display: flex; flex-direction: column; gap: 10px;">
                    <button
                        on:click=move |_| {
                            on_chat_open.run(());
                        }
                        style="
                            padding: 12px;
                            background: #444;
                            color: white;
                            border: none;
                            border-radius: 5px;
                            cursor: pointer;
                            text-align: left;
                            transition: background 0.2s;
                        "
                        onmouseover="this.style.background='#555'"
                        onmouseout="this.style.background='#444'"
                    >
                        "💬 Чат"
                    </button>

                    // Дополнительные кнопки можно добавить здесь
                    <button
                        style="
                            padding: 12px;
                            background: #444;
                            color: white;
                            border: none;
                            border-radius: 5px;
                            cursor: pointer;
                            text-align: left;
                            transition: background 0.2s;
                        "
                        onmouseover="this.style.background='#555'"
                        onmouseout="this.style.background='#444'"
                    >
                        "⚙️ Настройки"
                    </button>

                    <button
                        style="
                            padding: 12px;
                            background: #444;
                            color: white;
                            border: none;
                            border-radius: 5px;
                            cursor: pointer;
                            text-align: left;
                            transition: background 0.2s;
                        "
                        onmouseover="this.style.background='#555'"
                        onmouseout="this.style.background='#444'"
                    >
                        "📊 Статистика"
                    </button>
                </div>
            </div>
        </div>
    }
}
