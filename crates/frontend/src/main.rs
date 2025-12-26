mod components;
mod config;

use components::App;
use leptos::prelude::*;

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(|| view! { <App/> })
}
