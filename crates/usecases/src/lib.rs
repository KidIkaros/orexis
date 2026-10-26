//! Orexis use cases: the turn loop, context assembly, compaction, codec
//! selection, and replay verification. Depends only on core + ports.

mod actions;
mod codec;
mod compaction;
mod context;
mod replay;
mod session;
mod turn;

pub use codec::{select_codec, CodecCapabilities};
pub use compaction::{plan_compaction, ExtractiveSummarizer};
pub use context::{
    assemble, estimate_tokens, AssembledContext, ContextConfig, COMPACTION_BUFFER_TOKENS,
};
pub use replay::verify_replay;
pub use turn::{run_turn, TurnDeps, TurnOutcome};
