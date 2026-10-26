//! Left rail: tab shell and shared helpers.

mod files;
mod views;

use leptos::prelude::*;

use crate::state::{LeftTab, Snapshot};

use files::{artifacts_view, files_view};
use views::{board_view, fleet_view, sessions_view};

#[component]
pub fn LeftRail(
    snapshot: ReadSignal<Snapshot>,
    active: ReadSignal<LeftTab>,
    set_active: WriteSignal<LeftTab>,
    on_tab: Callback<LeftTab>,
    on_open_session: Callback<String>,
    on_list_dir: Callback<String>,
    on_read_file: Callback<String>,
    set_preview: WriteSignal<Option<String>>,
) -> impl IntoView {
    let (filter, set_filter) = signal(String::new());
    view! {
        <aside class="left panel">
            <div class="rail-tabs">
                {[
                    (LeftTab::Sessions, "sessions"),
                    (LeftTab::Board, "board*"),
                    (LeftTab::Fleet, "fleet*"),
                    (LeftTab::Artifacts, "changes"),
                    (LeftTab::Files, "files"),
                ]
                    .into_iter()
                    .map(|(tab, label)| {
                        let on_tab = on_tab.clone();
                        view! {
                            <button
                                class="rail-tab"
                                class:active=move || active.get() == tab
                                aria-pressed=move || active.get() == tab
                                on:click=move |_| {
                                    set_active.set(tab);
                                    on_tab.run(tab);
                                }
                            >
                                {label}
                            </button>
                        }
                    })
                    .collect_view()}
            </div>
            {move || match active.get() {
                LeftTab::Sessions => sessions_view(snapshot, on_open_session.clone()),
                LeftTab::Board => board_view(snapshot, on_open_session.clone()),
                LeftTab::Fleet => fleet_view(snapshot, on_open_session.clone()),
                LeftTab::Artifacts => {
                    artifacts_view(snapshot, set_preview, on_read_file.clone())
                }
                LeftTab::Files => {
                    files_view(
                        snapshot,
                        filter.get(),
                        set_filter,
                        on_list_dir.clone(),
                        on_read_file.clone(),
                    )
                }
            }}
        </aside>
    }
}

fn dot_class(status: &str) -> String {
    if status.contains("error") {
        "dot error"
    } else if status.contains("running") {
        "dot running"
    } else {
        "dot"
    }
    .to_string()
}

fn empty(snapshot: &Snapshot) -> AnyView {
    let message = if snapshot.connection == "attached" {
        "no sessions"
    } else {
        "connecting…"
    };
    view! { <p class="muted">{message}</p> }.into_any()
}
