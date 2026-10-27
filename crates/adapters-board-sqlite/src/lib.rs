//! SQLite board store (M8-02, ADR-0020): durable tasks with owner,
//! required strategy, and blocking edges. The contracts are encoded
//! against `FakeBoard` (adapters-fake): edges gate readiness, the cap
//! refuses visibly, transitions are ordered, reopen preserves.

mod migrate;
mod store;
mod util;

pub use store::SqliteBoard;
