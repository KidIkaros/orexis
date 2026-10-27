//! SQLite bus store (M8-03, ADR-0020): direct and room messages,
//! persisted as events. The contracts are encoded against `FakeBus`:
//! seq-ordered, worker-filtered reads, capped text, reopen preserves.

mod store;
mod util;

pub use store::SqliteBus;
