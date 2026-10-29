//! Right panel: contextual approvals, hunk-aligned diff, and file/output preview.

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
    let refresh_diff = diff.clone();
    view! {
        <aside class="right panel" aria-label="Contextual oversight">
            {move || {
                snapshot.get().pending.map(|approval| {
                    let id = approval.id;
                    let note = if matches!(
                        approval.tool.as_str(),
                        "write_file" | "edit_file"
                    ) {
                        "reversible: yes · git checkpoint available".to_string()
                    } else if approval.tool == "shell" {
                        "reversible: workspace files only · external effects persist".to_string()
                    } else {
                        "reversible: no".to_string()
                    };
                    let decide = decide.clone();
                    let diff = diff.clone();
                    view! {
                        <section class="approval-section" aria-live="assertive">
                            <div class="section-head">
                                <h2>"Needs Approval"</h2>
                                <span class="badge">"action required"</span>
                            </div>
                            <div class="approval card">
                                <div class="approval-head">
                                    <span class="tool-name">{approval.tool.clone()}</span>
                                    <span class="muted">"policy: ask"</span>
                                </div>
                                <pre class="approval-args">{approval.args.clone()}</pre>
                                <p class="reversibility-note">{note}</p>
                                <div class="approval-actions">
                                    <button
                                        class="approve"
                                        disabled=deciding
                                        on:click=move |_| decide.run((id, true, false))
                                    >
                                        "Approve "
                                        <kbd class="kbd kbd-dark">"y"</kbd>
                                    </button>
                                    <button
                                        class="deny"
                                        disabled=deciding
                                        on:click=move |_| decide.run((id, false, false))
                                    >
                                        "Deny "
                                        <kbd class="kbd">"n"</kbd>
                                    </button>
                                    <button
                                        class="always"
                                        title="Allow this tool for the rest of this session"
                                        disabled=deciding
                                        on:click=move |_| decide.run((id, true, true))
                                    >
                                        "Always "
                                        <kbd class="kbd">"a"</kbd>
                                    </button>
                                    <button class="diff" on:click=move |_| diff.run(())>
                                        "Diff "
                                        <kbd class="kbd">"d"</kbd>
                                    </button>
                                </div>
                            </div>
                        </section>
                    }
                })
            }}
            <div class="section-head">
                <h2>"Diff Inspector"</h2>
                <div class="diff-tools">
                    <button
                        class="tool-btn"
                        title="Refresh latest checkpoint diff (d)"
                        on:click=move |_| refresh_diff.run(())
                    >
                        "refresh "
                        <kbd class="kbd">"d"</kbd>
                    </button>
                    {move || {
                        snapshot.get().diff.is_some().then(|| {
                            let split_mode = split.get();
                            view! {
                                <>
                                    <button
                                        class="tool-btn"
                                        on:click=move |_| set_split.update(|value| *value = !*value)
                                    >
                                        {if split_mode { "unified" } else { "split" }}
                                    </button>
                                    <button
                                        class="tool-btn"
                                        on:click=move |_| set_expanded.update(|value| *value = !*value)
                                    >
                                        {if expanded.get() { "close" } else { "expand" }}
                                    </button>
                                </>
                            }
                        })
                    }}
                </div>
            </div>
            {move || match snapshot.get().diff {
                Some(text) => {
                    let split_mode = split.get();
                    view! {
                        <div class="diff-panel" class:full=move || expanded.get()>
                            {move || {
                                expanded.get().then(|| {
                                    let split_now = split.get();
                                    view! {
                                        <div class="diff-full-bar">
                                            <span class="brand">"Diff Inspector (Full Screen)"</span>
                                            <div class="diff-tools">
                                                <button
                                                    class="tool-btn"
                                                    on:click=move |_| set_split.update(|v| *v = !*v)
                                                >
                                                    {if split_now { "switch to unified" } else { "switch to split" }}
                                                </button>
                                                <button
                                                    class="tool-btn"
                                                    on:click=move |_| set_expanded.set(false)
                                                >
                                                    "close full screen"
                                                </button>
                                            </div>
                                        </div>
                                    }
                                })
                            }}
                            {if split_mode { split_diff(text) } else { unified_diff(text) }}
                        </div>
                    }
                        .into_any()
                }
                None => view! {
                    <p class="muted empty-hint">
                        "No active diff. Press "
                        <kbd class="kbd">"d"</kbd>
                        " to inspect uncommitted workspace changes."
                    </p>
                }
                    .into_any(),
            }}
            <div class="section-head">
                <h2>"Inspector Preview"</h2>
            </div>
            {move || {
                if let Some(text) = preview.get() {
                    return view! { <pre class="preview">{text}</pre> }.into_any();
                }
                if let Some(file) = snapshot.get().preview {
                    let mut body = String::new();
                    for (index, line) in file.content.lines().enumerate() {
                        body.push_str(&format!("{:>4}  {line}\n", index + 1));
                    }
                    if file.truncated {
                        body.push_str("\n[truncated]\n");
                    }
                    return view! {
                        <div class="file-preview">
                            <p class="file-path">{file.path}</p>
                            <pre class="preview">{body}</pre>
                        </div>
                    }
                        .into_any();
                }
                match auto_preview() {
                    Some(text) => view! { <pre class="preview">{text}</pre> }.into_any(),
                    None => view! {
                        <p class="muted empty-hint">
                            "Select a file in Files (5) or click 'open in right panel' on any tool output."
                        </p>
                    }
                        .into_any(),
                }
            }}
        </aside>
    }
}
