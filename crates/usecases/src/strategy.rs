//! Strategy definitions (M8-05, ADR-0020, invariant 16): a strategy is a
//! use case — data plus a narrowing filter — never an agent type. The
//! four strategies shape a worker's prompt, toolset, verify policy, and
//! budget; the toolset filter can only narrow the permission gate's
//! verdicts, never widen (the floor survives YOLO).

use orexis_core::{BoardTask, CostBudget, ToolsetFilter, VerifyLevel};

/// The four strategies (CONTEXT.md: a use case, not an agent type).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrategyId {
    Plan,
    Build,
    Review,
    Mine,
}

impl StrategyId {
    /// The shipped strategies; the set board tasks may name.
    pub const ALL: [StrategyId; 4] = [Self::Plan, Self::Build, Self::Review, Self::Mine];

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
                "read_file",
                "list_dir",
                "search",
                "git_status",
                "git_diff",
                "board_add",
                "board_list",
            ]),
            verify: VerifyLevel::OnRisk,
            budget: BudgetDefaults {
                max_steps: 12,
                wall_clock_secs: 600,
            },
        },
        StrategyId::Build => Strategy {
            id,
            prompt_fragment: "You are implementing one board task. Plan the change, edit, \
                             verify, and finish. If the task is unclear, say so instead of \
                             guessing.",
            toolset: ToolsetFilter::open(),
            verify: VerifyLevel::OnRisk,
            budget: BudgetDefaults {
                max_steps: 32,
                wall_clock_secs: 1800,
            },
        },
        StrategyId::Review => Strategy {
            id,
            prompt_fragment: "You are reviewing a change. Read the diff and the touched \
                             code, report findings, and finish. Do not edit files.",
            toolset: ToolsetFilter::allow(&[
                "read_file",
                "list_dir",
                "search",
                "git_status",
                "git_diff",
                "shell",
            ]),
            verify: VerifyLevel::OnRisk,
            budget: BudgetDefaults {
                max_steps: 12,
                wall_clock_secs: 600,
            },
        },
        StrategyId::Mine => Strategy {
            id,
            prompt_fragment: "You are mining the repository for facts the team asked \
                             about: read, search, and report. Do not edit files.",
            toolset: ToolsetFilter::allow(&["read_file", "list_dir", "search", "git_status"]),
            verify: VerifyLevel::OnRisk,
            budget: BudgetDefaults {
                max_steps: 16,
                wall_clock_secs: 900,
            },
        },
    }
}

/// The OO-Harness configuration (ADR-0021): `BaseHarness` (`HarnessConfig::base()`)
/// holds the shared single-agent turn defaults; each `StrategyId` specializes
/// `assemble` (`prompt_fragment`, `context_limit`, `compaction_keep_tail`),
/// `toolset`, `verify` (`verify_on_done`), and `budget` (`budget`, `cost_budget`)
/// via functional update (`HarnessConfig { ..base }`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HarnessConfig {
    pub strategy: Option<StrategyId>,
    pub prompt_fragment: &'static str,
    pub toolset: ToolsetFilter,
    pub verify: VerifyLevel,
    pub verify_on_done: bool,
    pub budget: BudgetDefaults,
    pub cost_budget: CostBudget,
    pub context_limit: usize,
    pub compaction_keep_tail: usize,
}

impl HarnessConfig {
    /// The `BaseHarness` prototype (ADR-0021).
    pub fn base() -> Self {
        Self {
            strategy: None,
            prompt_fragment: "",
            toolset: ToolsetFilter::open(),
            verify: VerifyLevel::OnRisk,
            verify_on_done: true,
            budget: BudgetDefaults {
                max_steps: 16,
                wall_clock_secs: 600,
            },
            cost_budget: CostBudget::default(),
            context_limit: 32_768,
            compaction_keep_tail: 8,
        }
    }
}

impl Default for HarnessConfig {
    fn default() -> Self {
        Self::base()
    }
}

/// One completed execution of a harness class on a board task (ADR-0021),
/// consumed by the pure `select_harness` router.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionRecord {
    pub strategy: StrategyId,
    pub passed_verification: bool,
    pub tokens_used: u64,
    pub steps: usize,
}

/// Specialize `BaseHarness` (`HarnessConfig::base()`) for `id` (ADR-0021).
pub fn harness_for(id: StrategyId) -> HarnessConfig {
    let base = HarnessConfig::base();
    let strat = strategy(id);
    match id {
        StrategyId::Plan => HarnessConfig {
            strategy: Some(id),
            prompt_fragment: strat.prompt_fragment,
            toolset: strat.toolset,
            verify: strat.verify,
            verify_on_done: false,
            budget: strat.budget,
            cost_budget: CostBudget {
                max_tokens: Some(32_000),
                ..base.cost_budget
            },
            context_limit: 16_384,
            compaction_keep_tail: 4,
        },
        StrategyId::Build => HarnessConfig {
            strategy: Some(id),
            prompt_fragment: strat.prompt_fragment,
            toolset: strat.toolset,
            verify: strat.verify,
            verify_on_done: true,
            budget: strat.budget,
            cost_budget: CostBudget {
                max_tokens: Some(64_000),
                ..base.cost_budget
            },
            context_limit: 16_384,
            compaction_keep_tail: 4,
        },
        StrategyId::Review => HarnessConfig {
            strategy: Some(id),
            prompt_fragment: strat.prompt_fragment,
            toolset: strat.toolset,
            verify: strat.verify,
            verify_on_done: true,
            budget: strat.budget,
            cost_budget: CostBudget {
                max_tokens: Some(32_000),
                ..base.cost_budget
            },
            context_limit: 16_384,
            compaction_keep_tail: 4,
        },
        StrategyId::Mine => HarnessConfig {
            strategy: Some(id),
            prompt_fragment: strat.prompt_fragment,
            toolset: strat.toolset,
            verify: strat.verify,
            verify_on_done: false,
            budget: strat.budget,
            cost_budget: CostBudget {
                max_tokens: Some(32_000),
                ..base.cost_budget
            },
            context_limit: 12_288,
            compaction_keep_tail: 4,
        },
    }
}

/// Pure router over `ExecutionRecord`s (ADR-0021): selects the specialized
/// `HarnessConfig` for `task.required_strategy`, falling back to `BaseHarness`
/// context/budget/verify defaults when the strategy's recorded executions
/// regress (< 50% verification pass rate over at least 2 runs) while
/// preserving the strategy's `ToolsetFilter` floor (invariant 16).
pub fn select_harness(records: &[ExecutionRecord], task: &BoardTask) -> HarnessConfig {
    let Some(id) = StrategyId::parse(&task.required_strategy) else {
        return HarnessConfig::base();
    };
    let specialized = harness_for(id);
    let mut total = 0usize;
    let mut passed = 0usize;
    for record in records.iter().filter(|record| record.strategy == id) {
        total += 1;
        if record.passed_verification {
            passed += 1;
        }
    }
    if total >= 2 && passed * 2 < total {
        HarnessConfig {
            prompt_fragment: specialized.prompt_fragment,
            toolset: specialized.toolset,
            ..HarnessConfig::base()
        }
    } else {
        specialized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_four_strategies_exist_and_are_bounded() {
        for id in [
            StrategyId::Plan,
            StrategyId::Build,
            StrategyId::Review,
            StrategyId::Mine,
        ] {
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
    fn the_plan_strategy_can_write_the_board() {
        // M8-11: the plan strategy produces the task breakdown, so the
        // board tools must survive its toolset filter.
        let plan = strategy(StrategyId::Plan);
        assert!(plan.toolset.permits("board_add"));
        assert!(plan.toolset.permits("board_list"));
        // The other strategies keep a closed toolset without them.
        for id in [StrategyId::Review, StrategyId::Mine] {
            assert!(!strategy(id).toolset.permits("board_add"));
        }
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

    #[test]
    fn harness_specialization_and_regression_fallback() {
        let base = HarnessConfig::base();
        assert_eq!(base.strategy, None);
        assert_eq!(base.context_limit, 32_768);
        assert_eq!(base.compaction_keep_tail, 8);

        let build_task = orexis_core::BoardTask {
            id: "t-1".into(),
            title: "implement feature".into(),
            required_strategy: "build".into(),
            owner: None,
            status: orexis_core::TaskStatus::Queued,
            depends_on: Vec::new(),
            seq: 1,
        };
        let selected = select_harness(&[], &build_task);
        assert_eq!(selected.strategy, Some(StrategyId::Build));
        assert_eq!(selected.context_limit, 16_384);
        assert_eq!(selected.compaction_keep_tail, 4);
        assert!(selected.verify_on_done);
        assert_eq!(selected.cost_budget.max_tokens, Some(64_000));

        let plan_task = orexis_core::BoardTask {
            required_strategy: "plan".into(),
            ..build_task.clone()
        };
        let plan_selected = select_harness(&[], &plan_task);
        assert_eq!(plan_selected.strategy, Some(StrategyId::Plan));
        assert!(!plan_selected.verify_on_done);
        assert!(!plan_selected.toolset.permits("write_file"));

        // Two failed executions on plan trigger fallback to BaseHarness
        // context/budget while keeping the strategy's read-only toolset floor.
        let records = vec![
            ExecutionRecord {
                strategy: StrategyId::Plan,
                passed_verification: false,
                tokens_used: 4_000,
                steps: 6,
            },
            ExecutionRecord {
                strategy: StrategyId::Plan,
                passed_verification: false,
                tokens_used: 5_000,
                steps: 8,
            },
        ];
        let fallback = select_harness(&records, &plan_task);
        assert_eq!(fallback.strategy, None);
        assert_eq!(fallback.context_limit, base.context_limit);
        assert_eq!(fallback.compaction_keep_tail, base.compaction_keep_tail);
        assert!(!fallback.toolset.permits("write_file"));
    }
}
