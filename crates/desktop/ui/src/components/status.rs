//! Status bar: connection, state, active model, undo, reattach.

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
            <span class="chip status-chip" aria-live="polite">
                <span class=move || {
                    let s = snapshot.get().status;
                    if error.get().is_some() || s == "error" || s == "disconnected" {
                        "dot error"
                    } else if s == "running" || s == "streaming" {
                        "dot running"
                    } else {
                        "dot"
                    }
                }></span>
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
            <span class="chip">
                {move || snapshot.get().session.unwrap_or_else(|| "no session".to_string())}
            </span>
            {move || {
                snapshot.get().settings.map(|cfg| {
                    let plan = if cfg.plan_mode { " · plan:on" } else { "" };
                    view! {
                        <span class="chip model-chip" title="Active provider profile and model">
                            {format!("{} · {}{}", cfg.profile, cfg.model, plan)}
                        </span>
                    }
                })
            }}
            <span class="spacer"></span>
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
                        <button
                            class="undo"
                            title="Restore latest git checkpoint (Ctrl/Cmd+Z)"
                            on:click=move |_| undo.run(())
                        >
                            "undo "
                            <kbd class="kbd">"⌘Z"</kbd>
                        </button>
                    }
                })
            }}
            <button
                class="undo"
                aria-label="settings"
                title="View model, permissions, and privacy boundaries"
                on:click=move |_| settings.run(())
            >
                "settings"
            </button>
        </header>
    }
}
