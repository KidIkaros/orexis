//! The Orexis bench (M8-10, §10): breadth tasks run serial and team
//! through the real daemon path, judged deterministically, compared with
//! a paired bootstrap and a noise band — the data the team-mode gate
//! reads.

pub mod cli;
pub mod compare;
pub mod judge;
pub mod runner;
pub mod task;

pub use compare::{run_compare, CompareReport};
pub use judge::Judge;
pub use runner::{run_suite, Mode, RunResult};
pub use task::BenchTask;
