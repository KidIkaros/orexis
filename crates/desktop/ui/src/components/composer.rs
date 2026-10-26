//! Composer: prompt input, skip-verify, send.

use leptos::prelude::*;

#[component]
pub fn Composer(
    composer: ReadSignal<String>,
    set_composer: WriteSignal<String>,
    skip: ReadSignal<bool>,
    set_skip: WriteSignal<bool>,
    on_send: Callback<()>,
) -> impl IntoView {
    let key_send = on_send.clone();
    view! {
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
                        key_send.run(());
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
                <button class="send" on:click=move |_| on_send.run(())>
                    "send"
                </button>
            </div>
        </div>
    }
}
