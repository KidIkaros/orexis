//! Left rail: sessions, board, fleet, artifacts, files.

use leptos::prelude::*;

use crate::state::{Artifact, FileEntry, LeftTab, Snapshot};

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
                    (LeftTab::Board, "board"),
                    (LeftTab::Fleet, "fleet"),
                    (LeftTab::Artifacts, "artifacts"),
                    (LeftTab::Files, "files"),
                ]
                    .into_iter()
                    .map(|(tab, label)| {
                        let on_tab = on_tab.clone();
                        view! {
                            <button
                                class="rail-tab"
                                class:active=move || active.get() == tab
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

fn sessions_view(snapshot: ReadSignal<Snapshot>, open: Callback<String>) -> AnyView {
    let sessions = snapshot.get().sessions;
    if sessions.is_empty() {
        return empty(&snapshot.get());
    }
    sessions
        .into_iter()
        .map(|session| {
            let id = session.id.clone();
            let open = open.clone();
            let dot = dot_class(&session.status);
            view! {
                <button class="row" on:click=move |_| open.run(id.clone())>
                    <span class=dot></span>
                    {session.id}
                    <span class="muted">" · "{session.status}</span>
                </button>
            }
        })
        .collect_view()
        .into_any()
}

fn board_view(snapshot: ReadSignal<Snapshot>, open: Callback<String>) -> AnyView {
    let sessions = snapshot.get().sessions;
    if sessions.is_empty() {
        return empty(&snapshot.get());
    }
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

fn fleet_view(snapshot: ReadSignal<Snapshot>, open: Callback<String>) -> AnyView {
    let sessions = snapshot.get().sessions;
    if sessions.is_empty() {
        return empty(&snapshot.get());
    }
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

fn artifacts_view(
    snapshot: ReadSignal<Snapshot>,
    set_preview: WriteSignal<Option<String>>,
    read_file: Callback<String>,
) -> AnyView {
    let artifacts = snapshot.get().artifacts;
    if artifacts.is_empty() {
        return view! { <p class="muted">"no artifacts"</p> }.into_any();
    }
    artifacts
        .into_iter()
        .map(|artifact| artifact_row(artifact, set_preview, read_file.clone()))
        .collect_view()
        .into_any()
}

fn artifact_row(
    artifact: Artifact,
    set_preview: WriteSignal<Option<String>>,
    read_file: Callback<String>,
) -> AnyView {
    let id = artifact.id.clone();
    let detail = artifact.detail.clone();
    view! {
        <button
            class="row artifact-row"
            on:click=move |_| {
                if detail.is_empty() {
                    read_file.run(id.clone());
                } else {
                    set_preview.set(Some(detail.clone()));
                }
            }
        >
            <span class="artifact-kind">{artifact.kind}</span>
            <span>{artifact.title}</span>
        </button>
    }
    .into_any()
}

fn files_view(
    snapshot: ReadSignal<Snapshot>,
    filter: String,
    set_filter: WriteSignal<String>,
    list_dir: Callback<String>,
    read_file: Callback<String>,
) -> AnyView {
    let state = snapshot.get();
    let path = state.files_path;
    let parent = parent_path(&path);
    let list = list_dir.clone();
    let entries = state.files;
    let filtered = entries
        .into_iter()
        .filter(|entry| {
            filter.is_empty() || entry.name.to_lowercase().contains(&filter.to_lowercase())
        })
        .collect::<Vec<_>>();
    view! {
        <div class="path-row">
            <span class="path">{path.clone()}</span>
            <button
                class="open"
                disabled=path == "."
                on:click=move |_| list.run(parent.clone())
            >
                "up"
            </button>
        </div>
        <input
            class="filter"
            placeholder="filter…"
            prop:value=filter
            on:input=move |ev| set_filter.set(event_target_value(&ev))
        />
        {if filtered.is_empty() {
            view! { <p class="muted">"empty directory"</p> }.into_any()
        } else {
            filtered
                .into_iter()
                .map(|entry| file_row(entry, list_dir.clone(), read_file.clone()))
                .collect_view()
                .into_any()
        }}
    }
    .into_any()
}

fn file_row(entry: FileEntry, list_dir: Callback<String>, read_file: Callback<String>) -> AnyView {
    let path = entry.path.clone();
    let is_dir = entry.is_dir;
    let size = entry.size;
    view! {
        <button
            class="row file-row"
            on:click=move |_| {
                if is_dir {
                    list_dir.run(path.clone());
                } else {
                    read_file.run(path.clone());
                }
            }
        >
            <span class=if is_dir { "file-dir" } else { "file-name" }>
                {entry.name}
            </span>
            <span class="muted file-size">
                {if is_dir { String::new() } else { size.to_string() }}
            </span>
        </button>
    }
    .into_any()
}

fn parent_path(path: &str) -> String {
    let parent = std::path::Path::new(path).parent();
    parent
        .and_then(|parent| parent.to_str())
        .filter(|parent| !parent.is_empty())
        .unwrap_or(".")
        .to_string()
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
