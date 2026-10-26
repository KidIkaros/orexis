//! Orexis use cases: the turn loop, context assembly, compaction, codec
//! selection, and replay verification. Depends only on core + ports.

mod actions;
mod codec;
mod compaction;
mod context;
mod entity;
mod identity;
mod memory;
mod replay;
mod session;
mod turn;

pub use codec::{select_codec, CodecCapabilities};
pub use compaction::{plan_compaction, ExtractiveSummarizer};
pub use context::{
    assemble, estimate_tokens, AssembledContext, ContextConfig, COMPACTION_BUFFER_TOKENS,
};
pub use entity::{decay, modulated_drives, posture, proposals, record_affect, record_outcome};
pub use identity::{append_diary, decay_traits, distill_trait, identity, self_view};
pub use memory::{forget, recall, remember};
pub use replay::verify_replay;
pub use turn::{run_turn, TurnDeps, TurnOutcome};
