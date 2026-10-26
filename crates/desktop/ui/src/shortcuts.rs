//! Keyboard shortcuts: approvals, diff, undo.

use leptos::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use crate::state::{LeftTab, Snapshot};

pub fn start_shortcuts(
    snapshot: ReadSignal<Snapshot>,
    decide: Callback<(u64, bool, bool)>,
    diff: Callback<()>,
    undo: Callback<()>,
    left: WriteSignal<LeftTab>,
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
            let tab = match event.key().as_str() {
                "1" => Some(LeftTab::Sessions),
                "2" => Some(LeftTab::Board),
                "3" => Some(LeftTab::Fleet),
                "4" => Some(LeftTab::Artifacts),
                "5" => Some(LeftTab::Files),
                "6" => Some(LeftTab::SelfView),
                _ => None,
            };
            if let Some(tab) = tab {
                event.prevent_default();
                left.set(tab);
                return;
            }
            if (event.ctrl_key() || event.meta_key()) && event.key().eq_ignore_ascii_case("z") {
                event.prevent_default();
                undo.run(());
                return;
            }
            if event.key() == "d" {
                event.prevent_default();
                diff.run(());
                return;
            }
            let Some(approval) = snapshot.get().pending else {
                return;
            };
            match event.key().as_str() {
                "y" => {
                    event.prevent_default();
                    decide.run((approval.id, true, false));
                }
                "n" => {
                    event.prevent_default();
                    decide.run((approval.id, false, false));
                }
                "a" => {
                    event.prevent_default();
                    decide.run((approval.id, true, true));
                }
                _ => {}
            }
        });
    if let Some(window) = web_sys::window() {
        let _ =
            window.add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref());
    }
    closure.forget();
}
