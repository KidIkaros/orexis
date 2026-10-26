//! Right panel: approvals, diff, preview.

use leptos::prelude::*;

use crate::diff::{split_diff, unified_diff};
use crate::state::{Entry, Snapshot};

#[component]
pub fn RightPanel(
    snapshot: ReadSignal<Snapshot>,
    preview: ReadSignal<Option<String>>,
    split: ReadSignal<bool>,
    set_split: WriteSignal<bool>,
    expanded: ReadSignal<bool>,
    set_expanded: WriteSignal<bool>,
    deciding: ReadSignal<bool>,
    on_decide: Callback<(u64, bool, bool)>,
    on_diff: Callback<()>,
) -> impl IntoView {
    let auto_preview = move || {
        snapshot
            .get()
            .transcript
            .iter()
            .rev()
            .find_map(|entry| match entry {
                Entry::Tool { detail, .. } if detail.len() > 2000 => Some(detail.clone()),
                _ => None,
            })
    };
    let decide = on_decide;
    let diff = on_diff;
    view! {
        <aside class="right panel">
            <h2>"APPROVALS"</h2>
            {move || match snapshot.get().pending {
                Some(approval) => {
                    let id = approval.id;
                    let note = if matches!(
                        approval.tool.as_str(),
                        "write_file" | "edit_file"
                    ) {
                        "reversible: yes · undo available".to_string()
                    } else if approval.tool == "shell" {
                        "reversible: files only · external effects persist".to_string()
                    } else {
                        "reversible: no".to_string()
                    };
                    let decide = decide.clone();
                    let diff = diff.clone();
                    view! {
                        <div class="approval card">
                            <div class="approval-head">
                                <span class="badge">"needs approval"</span>
                                <span class="muted">"risk: ask"</span>
                            </div>
                            <p class="tool-name">{approval.tool.clone()}</p>
                            <pre class="approval-args">{approval.args.clone()}</pre>
                            <p class="muted">{note}</p>
                            <div class="approval-actions">
                                <button
                                    class="approve"
                                    disabled=deciding
                                    on:click=move |_| decide.run((id, true, false))
                                >
                                    "approve (y)"
                                </button>
                                <button
                                    class="deny"
                                    disabled=deciding
                                    on:click=move |_| decide.run((id, false, false))
                                >
                                    "deny (n)"
                                </button>
                                <button
                                    class="always"
                                    title="allow this tool for this session"
                                    disabled=deciding
                                    on:click=move |_| decide.run((id, true, true))
                                >
                                    "always (a)"
                                </button>
                                <button class="diff" on:click=move |_| diff.run(())>
                                    "current diff (d)"
                                </button>
                            </div>
                        </div>
                    }
                        .into_any()
                }
                None => view! { <p class="muted">"none"</p> }.into_any(),
            }}
            <h2>"DIFF"</h2>
            {move || match snapshot.get().diff {
                Some(text) => {
                    let split_mode = split.get();
                    view! {
                        <div class="diff-panel" class:full=move || expanded.get()>
                            <div class="diff-tools">
                                <button
                                    class="open"
                                    on:click=move |_| set_split.update(|value| *value = !*value)
                                >
                                    {if split_mode { "unified" } else { "split" }}
                                </button>
                                <button
                                    class="open"
                                    on:click=move |_| set_expanded.update(|value| *value = !*value)
                                >
                                    {if expanded.get() { "close" } else { "expand" }}
                                </button>
                            </div>
                            {if split_mode { split_diff(text) } else { unified_diff(text) }}
                        </div>
                    }
                        .into_any()
                }
                None => view! { <p class="muted">"no diff"</p> }.into_any(),
            }}
            <h2>"PREVIEW"</h2>
            {move || match preview.get().or_else(auto_preview) {
                Some(text) => view! { <pre class="preview">{text}</pre> }.into_any(),
                None => view! { <p class="muted">"—"</p> }.into_any(),
            }}
        </aside>
    }
}
