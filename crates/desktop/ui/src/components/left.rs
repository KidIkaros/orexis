//! Left rail: sessions, board, fleet, artifacts, files.

use leptos::prelude::*;

use crate::state::{LeftTab, Snapshot};

#[component]
pub fn LeftRail(
    snapshot: ReadSignal<Snapshot>,
    active: ReadSignal<LeftTab>,
    set_active: WriteSignal<LeftTab>,
    on_open_session: Callback<String>,
) -> impl IntoView {
    let open = on_open_session;
    view! {
        <aside class="left panel">
            <div class="rail-tabs">
                {[
                    (LeftTab::Sessions, "sessions"),
                    (LeftTab::Board, "board"),
                    (LeftTab::Fleet, "fleet"),
                    (LeftTab::Artifacts, "artifacts"),
                    (LeftTab::Files, "files"),
                ]
                    .into_iter()
                    .map(|(tab, label)| {
                        view! {
                            <button
                                class="rail-tab"
                                class:active=move || active.get() == tab
                                on:click=move |_| set_active.set(tab)
                            >
                                {label}
                            </button>
                        }
                    })
                    .collect_view()}
            </div>
            {move || match active.get() {
                LeftTab::Sessions => {
                    let sessions = snapshot.get().sessions;
                    if sessions.is_empty() {
                        empty(&snapshot.get())
                    } else {
                        sessions
                            .into_iter()
                            .map(|session| {
                                let id = session.id.clone();
                                let open = open.clone();
                                let dot = dot_class(&session.status);
                                view! {
                                    <button
                                        class="row"
                                        on:click=move |_| open.run(id.clone())
                                    >
                                        <span class=dot></span>
                                        {session.id}
                                        <span class="muted">
                                            " · "
                                            {session.status}
                                        </span>
                                    </button>
                                }
                            })
                            .collect_view()
                            .into_any()
                    }
                }
                LeftTab::Board => {
                    let sessions = snapshot.get().sessions;
                    if sessions.is_empty() {
                        empty(&snapshot.get())
                    } else {
                        view! {
                            <div class="table">
                                <div class="table-head board">
                                    <span>"task"</span>
                                    <span>"stage"</span>
                                    <span>"worker"</span>
                                    <span>"blocked"</span>
                                </div>
                                {sessions
                                    .into_iter()
                                    .map(|session| {
                                        let id = session.id.clone();
                                        let task = session.id.clone();
                                        let open = open.clone();
                                        view! {
                                            <button
                                                class="row table-row board"
                                                on:click=move |_| open.run(id.clone())
                                            >
                                                <span>{task}</span>
                                                <span>{session.status}</span>
                                                <span>{session.id}</span>
                                                <span class="muted">"—"</span>
                                            </button>
                                        }
                                    })
                                    .collect_view()}
                            </div>
                        }
                            .into_any()
                    }
                }
                LeftTab::Fleet => {
                    let sessions = snapshot.get().sessions;
                    if sessions.is_empty() {
                        empty(&snapshot.get())
                    } else {
                        view! {
                            <div class="table">
                                <div class="table-head fleet">
                                    <span>"worker"</span>
                                    <span>"role"</span>
                                    <span>"model"</span>
                                    <span>"state"</span>
                                    <span>"task"</span>
                                    <span>"cost"</span>
                                </div>
                                {sessions
                                    .into_iter()
                                    .map(|session| {
                                        let id = session.id.clone();
                                        let worker = session.id.clone();
                                        let open = open.clone();
                                        view! {
                                            <button
                                                class="row table-row fleet"
                                                on:click=move |_| open.run(id.clone())
                                            >
                                                <span>{worker}</span>
                                                <span>"session"</span>
                                                <span class="muted">"—"</span>
                                                <span>{session.status}</span>
                                                <span class="muted">"—"</span>
                                                <span class="muted">"—"</span>
                                            </button>
                                        }
                                    })
                                    .collect_view()}
                            </div>
                        }
                            .into_any()
                    }
                }
                LeftTab::Artifacts => {
                    view! { <p class="muted">"artifacts land in M2-05"</p> }.into_any()
                }
                LeftTab::Files => {
                    view! { <p class="muted">"files land in M2-05"</p> }.into_any()
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
