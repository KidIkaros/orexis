//! App orchestrator: signals, callbacks, layout.

use leptos::prelude::*;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;

use crate::components::{Composer, LeftRail, RightPanel, SettingsModal, StatusBar, Transcript};
use crate::ipc::{args, invoke, start_listener};
use crate::shortcuts::start_shortcuts;
use crate::state::{Entry, LeftTab, Snapshot};

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
    let (left, set_left) = signal(LeftTab::Sessions);
    let (settings_open, set_settings_open) = signal(false);

    let attach = Callback::new(move |session: Option<String>| {
        spawn_local(async move {
            match invoke(
                "attach",
                args(serde_json::json!({"socket": "/tmp/orexis.sock", "session": session})),
            )
            .await
            {
                Ok(_) => {
                    set_error.set(None);
                    let _ = invoke("artifacts", JsValue::NULL).await;
                    let _ = invoke("settings", JsValue::NULL).await;
                    let _ = invoke("self_view", JsValue::NULL).await;
                    let _ = invoke("list_dir", args(serde_json::json!({"path": "."}))).await;
                }
                Err(err) => set_error.set(Some(
                    err.as_string()
                        .unwrap_or_else(|| "daemon unavailable".to_string()),
                )),
            }
        });
    });
    attach.run(None);
    let retry_attach = attach.clone();
    let retry = Callback::new(move |_: ()| retry_attach.run(None));
    let open_attach = attach.clone();
    let open_session = Callback::new(move |id: String| open_attach.run(Some(id)));

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

    let settings = Callback::new(move |_: ()| {
        spawn_local(async move {
            let _ = invoke("settings", JsValue::NULL).await;
        });
    });
    let open_settings = Callback::new({
        let settings = settings.clone();
        move |_: ()| {
            settings.run(());
            set_settings_open.set(true);
        }
    });
    let close_settings = Callback::new(move |_: ()| set_settings_open.set(false));

    let list_dir = Callback::new(move |path: String| {
        spawn_local(async move {
            let _ = invoke("list_dir", args(serde_json::json!({"path": path}))).await;
        });
    });
    let read_file = Callback::new(move |path: String| {
        set_preview.set(None);
        spawn_local(async move {
            let _ = invoke("read_file", args(serde_json::json!({"path": path}))).await;
        });
    });
    let artifacts = Callback::new(move |_: ()| {
        spawn_local(async move {
            let _ = invoke("artifacts", JsValue::NULL).await;
        });
    });
    let on_tab = Callback::new({
        let list_dir = list_dir.clone();
        let artifacts = artifacts.clone();
        move |tab: LeftTab| match tab {
            LeftTab::Files => {
                list_dir.run(".".to_string());
                artifacts.run(());
            }
            LeftTab::Artifacts => artifacts.run(()),
            LeftTab::Board => {
                spawn_local(async move {
                    let _ = invoke("board", JsValue::NULL).await;
                });
            }
            LeftTab::Fleet => {
                spawn_local(async move {
                    let _ = invoke("fleet", JsValue::NULL).await;
                });
            }
            _ => {}
        }
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

    Effect::new(move |_| {
        let snap = snapshot.get();
        if snap.diff.is_some() {
            set_expanded.set(snap.diff_expanded);
        } else {
            set_expanded.set(false);
        }
    });

    start_shortcuts(
        snapshot,
        decide.clone(),
        diff.clone(),
        undo.clone(),
        set_left,
    );
    view! {
        <div class="app">
            <StatusBar
                snapshot
                error=error
                on_attach=retry.clone()
                on_undo=undo.clone()
                on_settings=open_settings
            />
            <main class="zones">
                <LeftRail
                    snapshot
                    active=left
                    set_active=set_left
                    on_tab=on_tab
                    on_open_session=open_session
                    on_list_dir=list_dir
                    on_read_file=read_file
                    set_preview
                />
                <section class="center">
                    {move || {
                        error
                            .get()
                            .map(|message| {
                                view! {
                                    <div class="banner error-banner" role="alert">
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
                    {move || {
                        let inline_decide = decide.clone();
                        snapshot.get().pending.map(|approval| {
                            let id = approval.id;
                            view! {
                                <div class="banner approval-banner" role="status" aria-live="polite">
                                    <span>
                                        "⚠ Waiting on approval: "
                                        <strong>{approval.tool}</strong>
                                        " — review in the right panel or press "
                                        <kbd class="kbd">"y"</kbd>
                                        " / "
                                        <kbd class="kbd">"n"</kbd>
                                    </span>
                                    <div class="banner-actions">
                                        <button
                                            class="approve compact"
                                            on:click=move |_| inline_decide.run((id, true, false))
                                        >
                                            "Approve (y)"
                                        </button>
                                        <button
                                            class="deny compact"
                                            on:click=move |_| inline_decide.run((id, false, false))
                                        >
                                            "Deny (n)"
                                        </button>
                                    </div>
                                </div>
                            }
                        })
                    }}
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
                settings_open
                    .get()
                    .then(|| view! { <SettingsModal snapshot on_close=close_settings /> })
            }}
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
                    .map(|message| view! { <div class="toast" role="status" aria-live="polite">{message}</div> })
            }}
        </div>
    }
}
