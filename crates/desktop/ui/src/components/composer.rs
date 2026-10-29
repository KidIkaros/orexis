//! Composer: prompt input, skip-verify mode guard, send.

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
                placeholder="Ask Orexis… (Enter to send, Shift+Enter for newline)"
                aria-label="prompt"
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
                <label class="skip" class:armed=move || skip.get()>
                    <input
                        type="checkbox"
                        prop:checked=skip
                        on:change=move |ev| set_skip.set(event_target_checked(&ev))
                    />
                    {move || {
                        if skip.get() {
                            "⚠ skip verify (armed)"
                        } else {
                            "skip verify"
                        }
                    }}
                </label>
                <div class="composer-right">
                    <span class="hint">"⇧Enter newline · 1–6 panels · d diff"</span>
                    <button class="send" on:click=move |_| on_send.run(())>
                        "send "
                        <kbd class="kbd kbd-dark">"⏎"</kbd>
                    </button>
                </div>
            </div>
        </div>
    }
}
