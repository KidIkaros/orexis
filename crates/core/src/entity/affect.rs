//! PAD affect, derived moods, and posture selection.

use super::{Affect, DriveOutcome, Exploration, ModelTier, Posture, Risk, Thinking, VerifyLevel};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mood {
    Calm,
    Focused,
    Strained,
    Cautious,
    Curious,
    Restless,
    Neutral,
}

impl Mood {
    pub fn label(self) -> &'static str {
        match self {
            Self::Calm => "calm",
            Self::Focused => "focused",
            Self::Strained => "strained",
            Self::Cautious => "cautious",
            Self::Curious => "curious",
            Self::Restless => "restless",
            Self::Neutral => "neutral",
        }
    }
}

impl Affect {
    pub fn after(self, outcome: DriveOutcome) -> Self {
        let (valence, arousal, dominance) = match outcome {
            DriveOutcome::TaskDone => (0.30, -0.20, 0.20),
            DriveOutcome::TaskFailed => (-0.30, 0.20, -0.20),
            DriveOutcome::ToolError => (-0.15, 0.15, -0.10),
            DriveOutcome::VerifyFailed => (-0.25, 0.20, -0.15),
            DriveOutcome::TurnSpent => (-0.05, 0.05, -0.02),
            DriveOutcome::UserCorrection => (-0.10, 0.05, -0.05),
            DriveOutcome::Idle => (0.05, -0.10, 0.02),
        };
        Self {
            valence: self.valence + valence,
            arousal: self.arousal + arousal,
            dominance: self.dominance + dominance,
        }
        .clamped()
    }

    pub fn mood(self) -> Mood {
        if self.dominance <= -0.3 {
            Mood::Cautious
        } else if self.valence <= -0.3 && self.arousal >= 0.4 {
            Mood::Strained
        } else if self.arousal >= 0.5 && self.valence <= 0.0 {
            Mood::Restless
        } else if self.valence >= 0.3 && self.arousal <= 0.2 {
            Mood::Calm
        } else if self.dominance >= 0.3 && self.arousal >= 0.2 {
            Mood::Focused
        } else if self.valence >= 0.2 && self.arousal >= 0.3 {
            Mood::Curious
        } else {
            Mood::Neutral
        }
    }
}

impl Posture {
    /// Affect may add caution, never remove it. There is no posture field that
    /// can weaken a permission or bypass plan mode.
    pub fn for_affect(affect: Affect) -> Self {
        let mut posture = Self::default();
        if affect.arousal >= 0.5 {
            posture.model_tier = ModelTier::Fast;
            posture.repair_budget = 1;
            posture.exploration = Exploration::Low;
        }
        if affect.valence <= -0.3 || affect.dominance <= -0.2 {
            posture.verify = VerifyLevel::Always;
            posture.risk = Risk::Cautious;
            posture.repair_budget = posture.repair_budget.min(1);
            posture.exploration = Exploration::Low;
            posture.thinking = Thinking::Auto;
        }
        if affect.valence >= 0.3 && affect.dominance >= 0.2 {
            posture.verify = VerifyLevel::OnRisk;
            posture.risk = Risk::Normal;
        }
        posture
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failures_are_bounded_and_cautious() {
        let mut affect = Affect::default();
        for _ in 0..10 {
            affect = affect.after(DriveOutcome::TaskFailed);
        }
        assert_eq!(affect.valence, -1.0);
        assert_eq!(affect.dominance, -1.0);
        let posture = Posture::for_affect(affect);
        assert_eq!(posture.verify, VerifyLevel::Always);
        assert_eq!(posture.risk, Risk::Cautious);
        assert_eq!(posture.exploration, Exploration::Low);
        assert!(posture.repair_budget <= 1);
        assert_eq!(affect.mood(), Mood::Cautious);
    }

    #[test]
    fn success_returns_to_normal() {
        let affect = Affect::default().after(DriveOutcome::TaskDone);
        let posture = Posture::for_affect(affect);
        assert_eq!(posture.verify, VerifyLevel::OnRisk);
        assert_eq!(posture.risk, Risk::Normal);
        assert_eq!(affect.mood(), Mood::Calm);
    }

    #[test]
    fn posture_has_no_permission_surface() {
        let posture = Posture::for_affect(Affect {
            valence: -1.0,
            arousal: 1.0,
            dominance: -1.0,
        });
        assert_eq!(posture.risk, Risk::Cautious);
        assert_eq!(posture.verify, VerifyLevel::Always);
    }
}
