//! Strategy definitions (M8-05, ADR-0020, invariant 16): a strategy is a
//! use case — data plus a narrowing filter — never an agent type. The
//! four strategies shape a worker's prompt, toolset, verify policy, and
//! budget; the toolset filter can only narrow the permission gate's
//! verdicts, never widen (the floor survives YOLO).

use orexis_core::{ToolsetFilter, VerifyLevel};

/// The four strategies (CONTEXT.md: a use case, not an agent type).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrategyId {
    Plan,
    Build,
    Review,
    Mine,
}

impl StrategyId {
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "plan" => Some(Self::Plan),
            "build" => Some(Self::Build),
            "review" => Some(Self::Review),
            "mine" => Some(Self::Mine),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Plan => "plan",
            Self::Build => "build",
            Self::Review => "review",
            Self::Mine => "mine",
        }
    }
}

/// A strategy's budget defaults: step and wall-clock caps for a worker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BudgetDefaults {
    pub max_steps: usize,
    pub wall_clock_secs: u64,
}

/// A strategy: the use case a worker runs (ADR-0020).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Strategy {
    pub id: StrategyId,
    /// The prompt fragment injected into the worker's system prompt.
    pub prompt_fragment: &'static str,
    pub toolset: ToolsetFilter,
    pub verify: VerifyLevel,
    pub budget: BudgetDefaults,
}

/// The four shipped strategies. The prompt fragments are bounded by
/// construction (const strings); the toolsets narrow to what the
/// strategy's work needs.
pub fn strategy(id: StrategyId) -> Strategy {
    match id {
        StrategyId::Plan => Strategy {
            id,
            prompt_fragment: "You are planning work for a team. Read the code, produce a \
                             task breakdown on the board, and finish. Do not edit files; \
                             do not run mutating commands.",
            toolset: ToolsetFilter::allow(&[
                "read_file", "list_dir", "search", "git_status", "git_diff",
            ]),
            verify: VerifyLevel::OnRisk,
            budget: BudgetDefaults { max_steps: 12, wall_clock_secs: 600 },
        },
        StrategyId::Build => Strategy {
            id,
            prompt_fragment: "You are implementing one board task. Plan the change, edit, \
                             verify, and finish. If the task is unclear, say so instead of \
                             guessing.",
            toolset: ToolsetFilter::open(),
            verify: VerifyLevel::OnRisk,
            budget: BudgetDefaults { max_steps: 32, wall_clock_secs: 1800 },
        },
        StrategyId::Review => Strategy {
            id,
            prompt_fragment: "You are reviewing a change. Read the diff and the touched \
                             code, report findings, and finish. Do not edit files.",
            toolset: ToolsetFilter::allow(&[
                "read_file", "list_dir", "search", "git_status", "git_diff", "shell",
            ]),
            verify: VerifyLevel::OnRisk,
            budget: BudgetDefaults { max_steps: 12, wall_clock_secs: 600 },
        },
        StrategyId::Mine => Strategy {
            id,
            prompt_fragment: "You are mining the repository for facts the team asked \
                             about: read, search, and report. Do not edit files.",
            toolset: ToolsetFilter::allow(&[
                "read_file", "list_dir", "search", "git_status",
            ]),
            verify: VerifyLevel::OnRisk,
            budget: BudgetDefaults { max_steps: 16, wall_clock_secs: 900 },
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_four_strategies_exist_and_are_bounded() {
        for id in [StrategyId::Plan, StrategyId::Build, StrategyId::Review, StrategyId::Mine] {
            let strategy = strategy(id);
            assert!(
                !strategy.prompt_fragment.is_empty() && strategy.prompt_fragment.len() < 1000,
                "{}: the fragment is bounded",
                strategy.id.name()
            );
            assert!(strategy.budget.max_steps > 0);
            assert!(strategy.budget.wall_clock_secs > 0);
        }
    }

    #[test]
    fn parse_round_trips_and_refuses_unknowns() {
        assert_eq!(StrategyId::parse("plan"), Some(StrategyId::Plan));
        assert_eq!(StrategyId::parse("mine"), Some(StrategyId::Mine));
        assert_eq!(StrategyId::parse("build"), Some(StrategyId::Build));
        assert_eq!(StrategyId::parse("review"), Some(StrategyId::Review));
        assert_eq!(StrategyId::parse("execute"), None);
        assert_eq!(StrategyId::parse(""), None);
    }

    #[test]
    fn the_toolset_filter_narrows_by_name_and_namespace() {
        let filter = ToolsetFilter::allow(&["read_file", "mcp"]);
        assert!(filter.permits("read_file"));
        assert!(filter.permits("mcp/server/tool"));
        assert!(!filter.permits("write_file"));
        assert!(!filter.permits("mcpx/lookalike/tool"));
        // An empty allowlist is open; the gate still applies.
        assert!(ToolsetFilter::open().permits("write_file"));
    }
}
