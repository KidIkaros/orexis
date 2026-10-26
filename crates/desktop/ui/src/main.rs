//! Leptos CSR shell: modules, then mount.

mod app;
mod components;
mod diff;
mod ipc;
mod shortcuts;
mod state;

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(app::App);
}
