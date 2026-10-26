//! App orchestrator: signals, callbacks, layout.

use leptos::prelude::*;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;

use crate::components::{Composer, LeftRail, RightPanel, StatusBar, Transcript};
use crate::ipc::{args, invoke, start_listener};
use crate::shortcuts::start_shortcuts;
use crate::state::{Entry, Snapshot};

#[component]
pub fn App() -> impl IntoView {
    let (snapshot, set_snapshot) = signal(Snapshot::default());
    start_listener(set_snapshot);

    let (composer, set_composer) = signal(String::new());
    let (skip, set_skip) = signal(false);
    let (preview, set_preview) = signal(Option::<String>::None);
    let (error, set_error) = signal(Option::<String>::None);
    let (deciding, set_deciding) = signal(false);
    let (split, set_split) = signal(false);
    let (expanded, set_expanded) = signal(false);

    let attach = Callback::new(move |_: ()| {
        spawn_local(async move {
            match invoke(
                "attach",
                args(serde_json::json!({"socket": "/tmp/orexis.sock", "session": null})),
            )
            .await
            {
                Ok(_) => set_error.set(None),
                Err(err) => set_error.set(Some(
                    err.as_string()
                        .unwrap_or_else(|| "daemon unavailable".to_string()),
                )),
            }
        });
    });
    attach.run(());

    let send = Callback::new(move |_: ()| {
        let text = composer.get();
        if text.trim().is_empty() {
            return;
        }
        let skip_verify = skip.get();
        set_composer.set(String::new());
        spawn_local(async move {
            let _ = invoke(
                "prompt",
                args(serde_json::json!({"text": text, "skipVerify": skip_verify})),
            )
            .await;
        });
    });

    let decide = Callback::new(move |(id, yes, always): (u64, bool, bool)| {
        set_deciding.set(true);
        spawn_local(async move {
            if invoke(
                "approve",
                args(serde_json::json!({"id": id, "yes": yes, "always": always})),
            )
            .await
            .is_err()
            {
                set_deciding.set(false);
            }
        });
    });

    let diff = Callback::new(move |_: ()| {
        spawn_local(async move {
            let _ = invoke("diff", JsValue::NULL).await;
        });
    });
    let undo = Callback::new(move |_: ()| {
        spawn_local(async move {
            let _ = invoke("undo", JsValue::NULL).await;
        });
    });

    Effect::new(move |_| {
        let _ = snapshot.get().pending.as_ref().map(|approval| approval.id);
        set_deciding.set(false);
    });

    start_shortcuts(snapshot, decide.clone(), diff.clone(), undo.clone());

    let retry = attach.clone();
    view! {
        <div class="app">
            <StatusBar
                snapshot
                error=error
                on_attach=attach.clone()
                on_undo=undo.clone()
            />
            <main class="zones">
                <LeftRail snapshot />
                <section class="center">
                    {move || {
                        error
                            .get()
                            .map(|message| {
                                view! {
                                    <div class="banner error-banner">
                                        <span>{message}</span>
                                        <button
                                            class="send retry"
                                            on:click=move |_| retry.run(())
                                        >
                                            "retry"
                                        </button>
                                    </div>
                                }
                            })
                    }}
                    <Transcript snapshot set_preview />
                    <Composer
                        composer
                        set_composer
                        skip
                        set_skip
                        on_send=send
                    />
                </section>
                <RightPanel
                    snapshot
                    preview
                    split
                    set_split
                    expanded
                    set_expanded
                    deciding
                    on_decide=decide
                    on_diff=diff
                />
            </main>
            {move || {
                snapshot
                    .get()
                    .transcript
                    .iter()
                    .rev()
                    .find_map(|entry| match entry {
                        Entry::Info { message }
                            if message.starts_with("restored checkpoint") =>
                        {
                            Some(message.clone())
                        }
                        _ => None,
                    })
                    .map(|message| view! { <div class="toast">{message}</div> })
            }}
        </div>
    }
}
