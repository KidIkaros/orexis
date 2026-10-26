//! Evolution tiers and rollback triggers.

use serde::{Deserialize, Serialize};

use super::MutationKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tier {
    #[default]
    Off,
    Skills,
    Quarantine,
    Core,
}

impl Tier {
    pub fn can_propose(self) -> bool {
        self != Self::Off
    }

    pub fn requires_human(self) -> bool {
        matches!(self, Self::Quarantine | Self::Core)
    }

    pub fn requires_signed(self) -> bool {
        self == Self::Core
    }

    pub fn allows_apply(self, kind: MutationKind, human_approved: bool, signed: bool) -> bool {
        match self {
            Self::Off => false,
            Self::Skills => kind == MutationKind::SkillContent,
            Self::Quarantine => human_approved,
            Self::Core => human_approved && signed && kind != MutationKind::SkillContent,
        }
    }

    pub fn auto_apply(self, kind: MutationKind) -> bool {
        self == Self::Skills && kind == MutationKind::SkillContent
    }

    pub fn auto_rollback(self, trigger: RollbackTrigger) -> bool {
        match trigger {
            RollbackTrigger::Regression => self == Self::Skills,
            RollbackTrigger::SafetyIncident
            | RollbackTrigger::ChainFailure
            | RollbackTrigger::SignatureFailure
            | RollbackTrigger::Unrecoverable
            | RollbackTrigger::VerifyFailed => true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RollbackTrigger {
    SafetyIncident,
    ChainFailure,
    SignatureFailure,
    Unrecoverable,
    VerifyFailed,
    Regression,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_off() {
        assert_eq!(Tier::default(), Tier::Off);
        assert!(!Tier::Off.can_propose());
        assert!(!Tier::Off.allows_apply(MutationKind::SkillContent, true, true));
    }

    #[test]
    fn skills_apply_only_skill_content() {
        assert!(Tier::Skills.allows_apply(MutationKind::SkillContent, false, false));
        assert!(!Tier::Skills.allows_apply(MutationKind::GeneParameter, true, true));
        assert!(Tier::Skills.auto_apply(MutationKind::SkillContent));
    }

    #[test]
    fn quarantine_needs_human_and_core_needs_signature() {
        assert!(!Tier::Quarantine.allows_apply(MutationKind::SkillContent, false, false));
        assert!(Tier::Quarantine.allows_apply(MutationKind::SkillContent, true, false));
        assert!(!Tier::Core.allows_apply(MutationKind::GeneParameter, true, false));
        assert!(Tier::Core.allows_apply(MutationKind::GeneParameter, true, true));
    }

    #[test]
    fn rollback_triggers_follow_tier() {
        assert!(Tier::Skills.auto_rollback(RollbackTrigger::Regression));
        assert!(!Tier::Core.auto_rollback(RollbackTrigger::Regression));
        assert!(Tier::Core.auto_rollback(RollbackTrigger::SafetyIncident));
    }
}
