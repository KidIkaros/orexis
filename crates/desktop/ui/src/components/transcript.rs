//! Transcript: entries plus the live streaming line.

use leptos::prelude::*;

use crate::state::{Entry, Snapshot};

#[component]
pub fn Transcript(
    snapshot: ReadSignal<Snapshot>,
    set_preview: WriteSignal<Option<String>>,
) -> impl IntoView {
    view! {
        <div class="transcript" role="log" aria-label="transcript">
            {move || {
                let entries = snapshot.get().transcript;
                if entries.is_empty() && snapshot.get().live.is_none() {
                    return view! {
                        <div class="empty-state">
                            <p class="empty-title">"Ready to steer"</p>
                            <p class="muted">
                                "Send a task below. Tool executions, file diffs, and reversible checkpoints will appear side-by-side."
                            </p>
                        </div>
                    }
                        .into_any();
                }
                entries
                    .into_iter()
                    .map(|entry| match entry {
                        Entry::User { text } => {
                            view! {
                                <div class="line user">
                                    <span class="who">"you"</span>
                                    <span class="msg-body">{text}</span>
                                </div>
                            }
                                .into_any()
                        }
                        Entry::Assistant { text } => {
                            view! {
                                <div class="line assistant">
                                    <span class="who">"orexis"</span>
                                    <span class="msg-body">{text}</span>
                                </div>
                            }
                                .into_any()
                        }
                        Entry::Tool {
                            name,
                            target,
                            ok,
                            detail,
                        } => {
                            let preview_text = detail.clone();
                            let status_badge = if ok { "✓ done" } else { "⟳ running…" };
                            let status_cls = if ok { "tool-status ok" } else { "tool-status pending" };
                            view! {
                                <details class="line tool" class:ok=ok>
                                    <summary>
                                        <span class="tool-name">{name}</span>
                                        <span class="tool-target">{target}</span>
                                        <span class=status_cls>{status_badge}</span>
                                    </summary>
                                    <pre>{detail}</pre>
                                    <button
                                        class="open"
                                        on:click=move |_| {
                                            set_preview.set(Some(preview_text.clone()))
                                        }
                                    >
                                        "open in right panel"
                                    </button>
                                </details>
                            }
                                .into_any()
                        }
                        Entry::Approval { tool, args } => {
                            view! {
                                <p class="line approval-line">
                                    "⚠ approval requested · "
                                    <strong>{tool}</strong>
                                    " · "
                                    <code>{args}</code>
                                </p>
                            }
                                .into_any()
                        }
                        Entry::Error { message } => {
                            view! { <p class="line error">"✕ "{message}</p> }.into_any()
                        }
                        Entry::Info { message } => {
                            view! { <p class="line info">"· "{message}</p> }.into_any()
                        }
                    })
                    .collect_view()
                    .into_any()
            }}
            {move || {
                snapshot
                    .get()
                    .live
                    .map(|text| {
                        view! {
                            <div class="line assistant live">
                                <span class="who">"orexis"</span>
                                <span class="msg-body">
                                    {text}
                                    <span class="caret"></span>
                                </span>
                            </div>
                        }
                    })
            }}
        </div>
    }
}
