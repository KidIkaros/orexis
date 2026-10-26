//! Leptos CSR shell: three zones, daemon events over Tauri IPC.

use leptos::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "event"], js_name = listen)]
    fn tauri_listen(event: &str, handler: &js_sys::Function) -> js_sys::Promise;
}

fn start_listener(set_lines: WriteSignal<Vec<String>>) {
    let closure = Closure::<dyn FnMut(JsValue)>::new(move |event: JsValue| {
        let payload = js_sys::Reflect::get(&event, &JsValue::from_str("payload"))
            .unwrap_or(JsValue::NULL);
        let text = js_sys::JSON::stringify(&payload)
            .ok()
            .and_then(|value| value.as_string())
            .unwrap_or_default();
        set_lines.update(|lines| {
            lines.push(text);
            if lines.len() > 200 {
                lines.remove(0);
            }
        });
    });
    let _ = tauri_listen("orexis://event", closure.as_ref().unchecked_ref());
    closure.forget();
}

#[component]
fn App() -> impl IntoView {
    let (lines, set_lines) = signal(Vec::<String>::new());
    start_listener(set_lines);

    view! {
        <div class="app">
            <header class="status">"orexis · connected · ctx 0% · plan:off"</header>
            <main class="zones">
                <aside class="left">
                    <h2>"SESSIONS"</h2>
                    <p>"● s1 running"</p>
                    <h2>"BOARD"</h2>
                    <p>"#12 rewrite store"</p>
                    <h2>"FLEET"</h2>
                    <p>"● 1 builder"</p>
                    <h2>"ARTIFACTS"</h2>
                    <h2>"FILES"</h2>
                </aside>
                <section class="center">
                    <div class="transcript">
                        {move || {
                            lines
                                .get()
                                .into_iter()
                                .map(|line| view! { <p>{line}</p> })
                                .collect_view()
                        }}
                    </div>
                    <textarea class="composer" placeholder="ask orexis…"></textarea>
                </section>
                <aside class="right">
                    <h2>"APPROVALS"</h2>
                    <p>"none"</p>
                    <h2>"DIFF"</h2>
                    <h2>"PREVIEW"</h2>
                </aside>
            </main>
        </div>
    }
}

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}
