//! Settings modal: model, permissions, privacy.

use leptos::prelude::*;

use crate::state::Snapshot;

#[component]
pub fn SettingsModal(snapshot: ReadSignal<Snapshot>, on_close: Callback<()>) -> impl IntoView {
    let close = on_close;
    view! {
        <div class="modal-backdrop" on:click=move |_| close.run(())>
            <div
                class="modal card"
                role="dialog"
                aria-modal="true"
                aria-label="Settings"
                tabindex="0"
                autofocus=true
                on:click=move |ev| ev.stop_propagation()
                on:keydown=move |ev| {
                    if ev.key() == "Escape" {
                        close.run(());
                    }
                }
            >
                <header class="modal-head">
                    <h2>"settings"</h2>
                    <button class="open" aria-label="close settings" on:click=move |_| close.run(())>
                        "close"
                    </button>
                </header>
                {move || match snapshot.get().settings {
                    Some(settings) => {
                        view! {
                            <section class="settings-section">
                                <h3>"model"</h3>
                                <p>{settings.profile}" · "{settings.model}</p>
                                <p class="muted">
                                    "plan mode: "
                                    {if settings.plan_mode { "on" } else { "off" }}
                                </p>
                            </section>
                            <section class="settings-section">
                                <h3>"permissions"</h3>
                                <ul>
                                    {settings
                                        .permissions
                                        .into_iter()
                                        .map(|line| view! { <li>{line}</li> })
                                        .collect_view()}
                                </ul>
                            </section>
                            <section class="settings-section">
                                <h3>"what leaves this machine"</h3>
                                <ul>
                                    {settings
                                        .privacy
                                        .into_iter()
                                        .map(|line| view! { <li>{line}</li> })
                                        .collect_view()}
                                </ul>
                            </section>
                        }
                            .into_any()
                    }
                    None => view! { <p class="muted">"loading settings…"</p> }.into_any(),
                }}
            </div>
        </div>
    }
}
