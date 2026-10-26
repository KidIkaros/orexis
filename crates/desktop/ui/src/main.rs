//! Leptos CSR shell: state snapshots over Tauri IPC.

use leptos::prelude::*;
use serde::Deserialize;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{spawn_local, JsFuture};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "event"], js_name = listen)]
    fn tauri_listen(event: &str, handler: &js_sys::Function) -> js_sys::Promise;

    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
    fn tauri_invoke(command: &str, args: JsValue) -> js_sys::Promise;
}

async fn invoke(command: &str, args: JsValue) -> Result<JsValue, JsValue> {
    JsFuture::from(tauri_invoke(command, args)).await
}

fn args(value: serde_json::Value) -> JsValue {
    js_sys::JSON::parse(&value.to_string()).unwrap_or(JsValue::NULL)
}

#[derive(Clone, Deserialize)]
struct SessionInfo {
    id: String,
    status: String,
}

#[derive(Clone, Deserialize)]
struct Approval {
    id: u64,
    tool: String,
    args: String,
}

#[derive(Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Entry {
    User {
        text: String,
    },
    Assistant {
        text: String,
    },
    Tool {
        name: String,
        target: String,
        ok: bool,
        detail: String,
    },
    Approval {
        tool: String,
        args: String,
    },
    Error {
        message: String,
    },
    Info {
        message: String,
    },
}

#[derive(Clone, Deserialize)]
struct Snapshot {
    connection: String,
    session: Option<String>,
    sessions: Vec<SessionInfo>,
    transcript: Vec<Entry>,
    pending: Option<Approval>,
    status: String,
    live: Option<String>,
}

impl Default for Snapshot {
    fn default() -> Self {
        Self {
            connection: "connecting".to_string(),
            session: None,
            sessions: Vec::new(),
            transcript: Vec::new(),
            pending: None,
            status: "empty".to_string(),
            live: None,
        }
    }
}

fn start_listener(set: WriteSignal<Snapshot>) {
    let closure = Closure::<dyn FnMut(JsValue)>::new(move |event: JsValue| {
        let payload =
            js_sys::Reflect::get(&event, &JsValue::from_str("payload")).unwrap_or(JsValue::NULL);
        let text = js_sys::JSON::stringify(&payload)
            .ok()
            .and_then(|value| value.as_string())
            .unwrap_or_default();
        if let Ok(snapshot) = serde_json::from_str::<Snapshot>(&text) {
            set.set(snapshot);
        }
    });
    let _ = tauri_listen("orexis://state", closure.as_ref().unchecked_ref());
    closure.forget();
}

fn status_label(status: &str) -> &str {
    match status {
        "empty" => "ready",
        "running" => "running",
        "streaming" => "streaming",
        "approval" => "approval",
        "error" => "error",
        "budget" => "budget",
        "disconnected" => "disconnected",
        "queued" => "queued",
        "compacting" => "compacting",
        _ => status,
    }
}

fn start_approval_keys(
    snapshot: ReadSignal<Snapshot>,
    decide: impl Fn(u64, bool, bool) + 'static,
    set_preview: WriteSignal<Option<String>>,
) {
    let closure =
        Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |event: web_sys::KeyboardEvent| {
            let target = event.target();
            let typing = target
                .as_ref()
                .and_then(|target| target.dyn_ref::<web_sys::HtmlTextAreaElement>())
                .is_some()
                || target
                    .as_ref()
                    .and_then(|target| target.dyn_ref::<web_sys::HtmlInputElement>())
                    .is_some();
            if typing {
                return;
            }
            let Some(approval) = snapshot.get().pending else {
                return;
            };
            match event.key().as_str() {
                "y" => decide(approval.id, true, false),
                "n" => decide(approval.id, false, false),
                "a" => decide(approval.id, true, true),
                "d" => set_preview.set(Some(format!("diff preview\n{}", approval.args))),
                _ => {}
            }
        });
    if let Some(window) = web_sys::window() {
        let _ =
            window.add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref());
    }
    closure.forget();
}

#[component]
fn App() -> impl IntoView {
    let (snapshot, set_snapshot) = signal(Snapshot::default());
    start_listener(set_snapshot);

    let (composer, set_composer) = signal(String::new());
    let (skip, set_skip) = signal(false);
    let (preview, set_preview) = signal(Option::<String>::None);
    let (error, set_error) = signal(Option::<String>::None);
    let (deciding, set_deciding) = signal(false);

    let attach = move || {
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
    };
    attach();

    let send = move || {
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
    };

    Effect::new(move |_| {
        let _ = snapshot.get().pending.as_ref().map(|approval| approval.id);
        set_deciding.set(false);
    });

    let decide = move |id: u64, yes: bool, always: bool| {
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
    };

    start_approval_keys(snapshot, decide, set_preview);

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

    view! {
        <div class="app">
            <header class="status">
                <span class="brand">"orexis"</span>
                <span class="chip">
                    {move || {
                        if error.get().is_some() {
                            "daemon unavailable".to_string()
                        } else {
                            format!(
                                "{} · {}",
                                snapshot.get().connection,
                                status_label(&snapshot.get().status),
                            )
                        }
                    }}
                </span>
                <span class="spacer"></span>
                {move || {
                    (snapshot.get().connection == "disconnected")
                        .then(|| {
                            view! {
                                <button class="send reattach" on:click=move |_| attach()>
                                    "reattach"
                                </button>
                            }
                        })
                }}
                <span class="meta">
                    {move || snapshot.get().session.unwrap_or_else(|| "no session".to_string())}
                </span>
            </header>

            <main class="zones">
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

                <section class="center">
                    {move || {
                        error.get().map(|message| {
                            view! {
                                <div class="banner error-banner">
                                    <span>{message}</span>
                                    <button class="send retry" on:click=move |_| attach()>
                                        "retry"
                                    </button>
                                </div>
                            }
                        })
                    }}
                    {move || {
                        let status = snapshot.get().status;
                        let label = status_label(&status).to_string();
                        matches!(
                            status.as_str(),
                            "error" | "budget" | "disconnected" | "queued" | "compacting"
                        )
                            .then(move || {
                                view! { <div class="banner">{label}</div> }
                            })
                    }}

                    <div class="transcript">
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

                    <div class="composer-wrap">
                        <textarea
                            class="composer"
                            autofocus=true
                            placeholder="ask orexis…"
                            prop:value=composer
                            on:input=move |ev| set_composer.set(event_target_value(&ev))
                            on:keydown=move |ev| {
                                if ev.key() == "Enter" && !ev.shift_key() {
                                    ev.prevent_default();
                                    send();
                                }
                            }
                        ></textarea>
                        <div class="composer-actions">
                            <label class="skip">
                                <input
                                    type="checkbox"
                                    prop:checked=skip
                                    on:change=move |ev| set_skip.set(event_target_checked(&ev))
                                />
                                "skip verify"
                            </label>
                            <button class="send" on:click=move |_| send()>
                                "send"
                            </button>
                        </div>
                    </div>
                </section>

                <aside class="right panel">
                    <h2>"APPROVALS"</h2>
                    {move || match snapshot.get().pending {
                        Some(approval) => {
                            let id = approval.id;
                            view! {
                                <div class="approval card">
                                    <div class="approval-head">
                                        <span class="badge">"needs approval"</span>
                                        <span class="muted">"risk: ask"</span>
                                    </div>
                                    <p class="tool-name">{approval.tool.clone()}</p>
                                    <pre class="approval-args">{approval.args.clone()}</pre>
                                    <p class="muted">"reversible: yes · undo available"</p>
                                    <div class="approval-actions">
                                        <button
                                            class="approve"
                                            disabled=deciding
                                            on:click=move |_| decide(id, true, false)
                                        >
                                            "approve (y)"
                                        </button>
                                        <button
                                            class="deny"
                                            disabled=deciding
                                            on:click=move |_| decide(id, false, false)
                                        >
                                            "deny (n)"
                                        </button>
                                        <button
                                            class="always"
                                            disabled=deciding
                                            on:click=move |_| decide(id, true, true)
                                        >
                                            "always (a)"
                                        </button>
                                        <button
                                            class="diff"
                                            on:click=move |_| {
                                                set_preview
                                                    .set(
                                                        Some(
                                                            format!("diff preview\n{}", approval.args.clone()),
                                                        ),
                                                    )
                                            }
                                        >
                                            "diff (d)"
                                        </button>
                                    </div>
                                </div>
                            }
                                .into_any()
                        }
                        None => view! { <p class="muted">"none"</p> }.into_any(),
                    }}
                    <h2>"DIFF"</h2>
                    <p class="muted">"—"</p>
                    <h2>"PREVIEW"</h2>
                    {move || match preview.get().or_else(auto_preview) {
                        Some(text) => view! { <pre class="preview">{text}</pre> }.into_any(),
                        None => view! { <p class="muted">"—"</p> }.into_any(),
                    }}
                </aside>
            </main>
        </div>
    }
}

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}
