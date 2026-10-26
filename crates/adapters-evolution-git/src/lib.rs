//! Git-backed evolution corridor with a signed, hash-chained ledger.

mod corridor;
mod git;
mod signer;

pub use corridor::{import_jsonl, GitCorridor};
pub use signer::Ed25519Signer;
