//! Tauri IPC: state snapshots and commands.

use leptos::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

use crate::state::Snapshot;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "event"], js_name = listen)]
    fn tauri_listen(event: &str, handler: &js_sys::Function) -> js_sys::Promise;

    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
    fn tauri_invoke(command: &str, args: JsValue) -> js_sys::Promise;
}

pub async fn invoke(command: &str, args: JsValue) -> Result<JsValue, JsValue> {
    JsFuture::from(tauri_invoke(command, args)).await
}

pub fn args(value: serde_json::Value) -> JsValue {
    js_sys::JSON::parse(&value.to_string()).unwrap_or(JsValue::NULL)
}

pub fn start_listener(set: WriteSignal<Snapshot>) {
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
