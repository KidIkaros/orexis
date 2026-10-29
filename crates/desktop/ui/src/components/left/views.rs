//! Session-derived board and fleet projections.

use leptos::prelude::*;

use crate::state::Snapshot;

use super::{dot_class, empty};

pub(super) fn sessions_view(snapshot: ReadSignal<Snapshot>, open: Callback<String>) -> AnyView {
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

pub(super) fn board_view(snapshot: ReadSignal<Snapshot>, open: Callback<String>) -> AnyView {
    let snap = snapshot.get();
    if let Some(board_text) = snap.board_text {
        return view! { <pre class="preview">{board_text}</pre> }.into_any();
    }
    let sessions = snap.sessions.clone();
    if sessions.is_empty() {
        return empty(&snap);
    }
    view! {
        <p class="muted">"session projection · team board ready"</p>
        <div class="table">
            <div class="table-head board">
                <span>"session"</span>
                <span>"stage"</span>
                <span>"owner"</span>
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

pub(super) fn fleet_view(snapshot: ReadSignal<Snapshot>, open: Callback<String>) -> AnyView {
    let snap = snapshot.get();
    if let Some(fleet_text) = snap.fleet_text {
        return view! { <pre class="preview">{fleet_text}</pre> }.into_any();
    }
    let sessions = snap.sessions.clone();
    if sessions.is_empty() {
        return empty(&snap);
    }
    view! {
        <p class="muted">"session projection · team fleet ready"</p>
        <div class="table">
            <div class="table-head fleet">
                <span>"session"</span>
                <span>"role"</span>
                <span>"model"</span>
                <span>"state"</span>
                <span>"activity"</span>
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
