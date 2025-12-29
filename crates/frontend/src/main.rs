use leptos::prelude::*;

mod components;
mod config;
mod i18n;

use components::App;

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(|| view! { <App/> })
}
