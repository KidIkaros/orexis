//! Status bar: connection, state, undo, reattach.

use leptos::prelude::*;

use crate::state::{status_label, Snapshot};

#[component]
pub fn StatusBar(
    snapshot: ReadSignal<Snapshot>,
    error: ReadSignal<Option<String>>,
    on_attach: Callback<()>,
    on_undo: Callback<()>,
    on_settings: Callback<()>,
) -> impl IntoView {
    let retry = on_attach;
    let undo = on_undo;
    let settings = on_settings;
    view! {
        <header class="status">
            <span class="brand">"orexis"</span>
            <span class="chip" aria-live="polite">
                {move || {
                    if error.get().is_some() {
                        "daemon unavailable".to_string()
                    } else {
                        format!(
                            "{} · {}",
                            snapshot.get().connection,
                            status_label(&snapshot.get().status),
                        )
                    }
                }}
            </span>
            <span class="spacer"></span>
            <button class="undo" aria-label="settings" on:click=move |_| settings.run(())>
                "settings"
            </button>
            {move || {
                (snapshot.get().connection == "disconnected")
                    .then(|| {
                        view! {
                            <button class="send reattach" on:click=move |_| retry.run(())>
                                "reattach"
                            </button>
                        }
                    })
            }}
            {move || {
                snapshot.get().session.is_some().then(|| {
                    view! {
                        <button class="undo" on:click=move |_| undo.run(())>
                            "undo"
                        </button>
                    }
                })
            }}
            <span class="meta">
                {move || snapshot.get().session.unwrap_or_else(|| "no session".to_string())}
            </span>
        </header>
    }
}
