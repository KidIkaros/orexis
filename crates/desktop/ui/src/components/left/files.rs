//! Changes and file browser views.

use leptos::prelude::*;

use crate::state::{Artifact, FileEntry, Snapshot};

pub(super) fn artifacts_view(
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

pub(super) fn files_view(
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
                aria-label="parent directory"
                disabled=path == "."
                on:click=move |_| list.run(parent.clone())
            >
                "up"
            </button>
        </div>
        <input
            class="filter"
            aria-label="filter files"
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
