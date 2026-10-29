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
        <aside class="left panel" aria-label="Workspace navigation">
            <div class="rail-tabs" role="tablist">
                {[
                    (LeftTab::Sessions, "1", "Sessions"),
                    (LeftTab::Board, "2", "Board"),
                    (LeftTab::Fleet, "3", "Fleet"),
                    (LeftTab::Artifacts, "4", "Changes"),
                    (LeftTab::Files, "5", "Files"),
                    (LeftTab::SelfView, "6", "Self"),
                ]
                    .into_iter()
                    .map(|(tab, key, label)| {
                        let on_tab = on_tab.clone();
                        view! {
                            <button
                                class="rail-tab"
                                role="tab"
                                class:active=move || active.get() == tab
                                aria-selected=move || active.get() == tab
                                on:click=move |_| {
                                    set_active.set(tab);
                                    on_tab.run(tab);
                                }
                            >
                                <span class="tab-label">{label}</span>
                                <kbd class="kbd">{key}</kbd>
                            </button>
                        }
                    })
                    .collect_view()}
            </div>
            <div class="rail-body">
                {move || match active.get() {
                    LeftTab::Sessions => sessions_view(snapshot, on_open_session.clone()),
                    LeftTab::Board => board_view(snapshot, on_open_session.clone()),
                    LeftTab::Fleet => fleet_view(snapshot, on_open_session.clone()),
                    LeftTab::Artifacts => {
                        artifacts_view(snapshot, set_preview, on_read_file.clone())
                    }
                    LeftTab::SelfView => self_view(snapshot),
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
            </div>
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

fn self_view(snapshot: ReadSignal<Snapshot>) -> AnyView {
    match snapshot.get().self_text {
        Some(text) => view! { <pre class="preview">{text}</pre> }.into_any(),
        None => view! { <p class="muted">"no entity state yet"</p> }.into_any(),
    }
}

fn empty(snapshot: &Snapshot) -> AnyView {
    let message = if snapshot.connection == "attached" {
        "no sessions"
    } else {
        "connecting…"
    };
    view! { <p class="muted">{message}</p> }.into_any()
}
