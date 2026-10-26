//! The vertical slice: prompt → tool call → session event → render → replay.

use futures::executor::block_on;
use orexis_adapters_fake::render_transcript;
use orexis_composition::build;
use orexis_core::derive_messages;
use orexis_usecases::{run_turn, verify_replay, TurnDeps};

fn main() {
    let app = build();

    let outcome = block_on(run_turn(TurnDeps {
        provider: app.provider.as_ref(),
        tools: app.tools.as_ref(),
        session: app.session.as_ref(),
        reader: app.reader.as_ref(),
        sink: app.sink.as_ref(),
        session_id: "slice-1",
        prompt: "read src/lib.rs and summarize",
        max_steps: 8,
    }));

    let events = block_on(app.reader.events());
    print!("{}", render_transcript(&events));
    println!("steps: {}  done: {}", outcome.steps, outcome.done);

    assert!(verify_replay(&events), "replay diverged");
    let messages = derive_messages(&events);
    println!("messages: {}  replay: byte-identical", messages.len());
}
