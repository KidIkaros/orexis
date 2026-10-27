//! The blackboard on the filesystem (M8-04, ADR-0020): durable outputs
//! as files under a jailed root, plus leases on workspace paths — an
//! owner and an expiry; a stale lease is stealable. Leases persist
//! (write-through JSON, atomic rename) so a daemon restart does not
//! forget who holds what.

mod store;

pub use store::FsBlackboard;
