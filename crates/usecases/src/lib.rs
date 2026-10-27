//! Orexis use cases: the turn loop, context assembly, compaction, codec
//! selection, and replay verification. Depends only on core + ports.

mod actions;
mod apply;
mod codec;
mod compaction;
mod context;
mod entity;
mod evolution;
mod identity;
mod memory;
mod replay;
mod scheduler;
mod session;
mod strategy;
mod supervisor;
mod turn;

pub use apply::{apply_candidate, handle_trigger, rollback, should_rollback, ApplyRequest};
pub use codec::{select_codec, CodecCapabilities};
pub use compaction::{plan_compaction, ExtractiveSummarizer};
pub use context::{
    assemble, estimate_tokens, AssembledContext, ContextConfig, COMPACTION_BUFFER_TOKENS,
};
pub use entity::{decay, modulated_drives, posture, proposals, record_affect, record_outcome};
pub use evolution::{growth_allows, propose_growth, GROWTH_THRESHOLD};
pub use identity::{append_diary, decay_traits, distill_trait, identity, self_view};
pub use memory::{forget, recall, remember};
pub use replay::verify_replay;
pub use scheduler::{admit_ready, ROSTER_CAP};
pub use strategy::{strategy, BudgetDefaults, Strategy, StrategyId};
pub use supervisor::{supervise, Supervision, SupervisorState, MAX_RESTARTS};
pub use turn::{run_turn, TurnDeps, TurnOutcome};
