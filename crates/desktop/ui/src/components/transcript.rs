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
                snapshot
                    .get()
                    .transcript
                    .into_iter()
                    .map(|entry| match entry {
                        Entry::User { text } => {
                            view! {
                                <p class="line user">
                                    <span class="who">"you › "</span>
                                    {text}
                                </p>
                            }
                                .into_any()
                        }
                        Entry::Assistant { text } => {
                            view! {
                                <p class="line assistant">
                                    <span class="who">"orexis › "</span>
                                    {text}
                                </p>
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
                            view! {
                                <details class="line tool" class:ok=ok>
                                    <summary>
                                        <span class="tool-name">{name}</span>
                                        " "
                                        {target}
                                    </summary>
                                    <pre>{detail}</pre>
                                    <button
                                        class="open"
                                        on:click=move |_| {
                                            set_preview.set(Some(preview_text.clone()))
                                        }
                                    >
                                        "open in panel"
                                    </button>
                                </details>
                            }
                                .into_any()
                        }
                        Entry::Approval { tool, args } => {
                            view! {
                                <p class="line approval-line">
                                    "approval requested · "
                                    {tool}
                                    " · "
                                    {args}
                                </p>
                            }
                                .into_any()
                        }
                        Entry::Error { message } => {
                            view! { <p class="line error">{message}</p> }.into_any()
                        }
                        Entry::Info { message } => {
                            view! { <p class="line info">{message}</p> }.into_any()
                        }
                    })
                    .collect_view()
            }}
            {move || {
                snapshot
                    .get()
                    .live
                    .map(|text| {
                        view! {
                            <p class="line assistant live">
                                <span class="who">"orexis › "</span>
                                {text}
                                <span class="caret"></span>
                            </p>
                        }
                    })
            }}
        </div>
    }
}
