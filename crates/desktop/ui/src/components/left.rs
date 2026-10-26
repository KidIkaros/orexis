//! Left rail: sessions and placeholder panels.

use leptos::prelude::*;

use crate::state::Snapshot;

#[component]
pub fn LeftRail(snapshot: ReadSignal<Snapshot>) -> impl IntoView {
    view! {
        <aside class="left panel">
            <h2>"SESSIONS"</h2>
            {move || {
                let sessions = snapshot.get().sessions;
                if sessions.is_empty() {
                    view! { <p class="muted">"no sessions"</p> }.into_any()
                } else {
                    sessions
                        .into_iter()
                        .map(|session| {
                            let dot = if session.status.contains("error") {
                                "dot error"
                            } else if session.status.contains("running") {
                                "dot running"
                            } else {
                                "dot"
                            }
                            .to_string();
                            view! {
                                <p class="session">
                                    <span class=dot></span>
                                    {session.id}
                                    " · "
                                    {session.status}
                                </p>
                            }
                        })
                        .collect_view()
                        .into_any()
                }
            }}
            <h2>"BOARD"</h2>
            <p class="muted">"—"</p>
            <h2>"FLEET"</h2>
            <p class="muted">"—"</p>
            <h2>"ARTIFACTS"</h2>
            <p class="muted">"—"</p>
            <h2>"FILES"</h2>
            <p class="muted">"—"</p>
        </aside>
    }
}
