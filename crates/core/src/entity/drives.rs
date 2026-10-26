//! Drive dynamics: decay, satisfaction, tension, proposals, and outcomes.

use super::Drives;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriveOutcome {
    TaskDone,
    TaskFailed,
    ToolError,
    VerifyFailed,
    TurnSpent,
    UserCorrection,
    Idle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriveProposal {
    Explore,
    Build,
    Refactor,
    Consolidate,
    Evolve,
    CheckIn,
}

impl DriveProposal {
    pub fn drive(self) -> super::Drive {
        match self {
            Self::Explore => super::Drive::Curiosity,
            Self::Build => super::Drive::Mastery,
            Self::Refactor => super::Drive::Coherence,
            Self::Consolidate => super::Drive::Rest,
            Self::Evolve => super::Drive::Growth,
            Self::CheckIn => super::Drive::Connection,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Explore => "explore",
            Self::Build => "build",
            Self::Refactor => "refactor",
            Self::Consolidate => "consolidate",
            Self::Evolve => "evolve",
            Self::CheckIn => "check-in",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DriveTension {
    pub a: super::Drive,
    pub b: super::Drive,
    pub label: &'static str,
}

impl super::Drive {
    pub const ALL: [Self; 6] = [
        Self::Connection,
        Self::Curiosity,
        Self::Mastery,
        Self::Coherence,
        Self::Rest,
        Self::Growth,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Connection => "connection",
            Self::Curiosity => "curiosity",
            Self::Mastery => "mastery",
            Self::Coherence => "coherence",
            Self::Rest => "rest",
            Self::Growth => "growth",
        }
    }
}

impl Drives {
    pub fn baseline() -> Self {
        Self::default()
    }

    pub fn decay(self, baseline: Self, rate: f32) -> Self {
        let rate = rate.clamp(0.0, 1.0);
        let mut next = self;
        for drive in super::Drive::ALL {
            let value = self.get(drive);
            let base = baseline.get(drive);
            next.set(drive, value + (base - value) * rate);
        }
        next
    }

    pub fn satisfy(mut self, drive: super::Drive, amount: f32) -> Self {
        self.set(drive, self.get(drive) - amount.max(0.0));
        self
    }

    pub fn after(self, outcome: DriveOutcome) -> Self {
        let mut next = self;
        let mut adjust = |drive, delta: f32| {
            next.set(drive, next.get(drive) + delta);
        };
        match outcome {
            DriveOutcome::TaskDone => {
                adjust(super::Drive::Mastery, 0.15);
                adjust(super::Drive::Rest, 0.10);
                adjust(super::Drive::Curiosity, -0.10);
                adjust(super::Drive::Growth, 0.05);
            }
            DriveOutcome::TaskFailed => {
                adjust(super::Drive::Mastery, -0.10);
                adjust(super::Drive::Coherence, 0.10);
                adjust(super::Drive::Rest, 0.05);
                adjust(super::Drive::Growth, 0.10);
            }
            DriveOutcome::ToolError => {
                adjust(super::Drive::Curiosity, 0.05);
                adjust(super::Drive::Mastery, -0.05);
                adjust(super::Drive::Coherence, 0.05);
            }
            DriveOutcome::VerifyFailed => {
                adjust(super::Drive::Mastery, -0.10);
                adjust(super::Drive::Coherence, 0.15);
                adjust(super::Drive::Growth, 0.05);
            }
            DriveOutcome::TurnSpent => {
                adjust(super::Drive::Rest, 0.05);
                adjust(super::Drive::Curiosity, 0.02);
            }
            DriveOutcome::UserCorrection => {
                adjust(super::Drive::Connection, 0.10);
                adjust(super::Drive::Coherence, 0.10);
                adjust(super::Drive::Growth, 0.05);
            }
            DriveOutcome::Idle => {
                adjust(super::Drive::Connection, 0.05);
                adjust(super::Drive::Rest, 0.05);
            }
        }
        next
    }

    pub fn tensions(&self) -> Vec<DriveTension> {
        let high = |drive| self.get(drive) >= 0.65;
        let low = |drive| self.get(drive) <= 0.35;
        let mut out = Vec::new();
        if high(super::Drive::Connection) && high(super::Drive::Rest) {
            out.push(DriveTension {
                a: super::Drive::Connection,
                b: super::Drive::Rest,
                label: "present but quiet",
            });
        }
        if high(super::Drive::Curiosity) && low(super::Drive::Mastery) {
            out.push(DriveTension {
                a: super::Drive::Curiosity,
                b: super::Drive::Mastery,
                label: "wants to understand but cannot",
            });
        }
        if high(super::Drive::Coherence) && low(super::Drive::Connection) {
            out.push(DriveTension {
                a: super::Drive::Coherence,
                b: super::Drive::Connection,
                label: "needs to integrate alone",
            });
        }
        if high(super::Drive::Growth) && high(super::Drive::Rest) {
            out.push(DriveTension {
                a: super::Drive::Growth,
                b: super::Drive::Rest,
                label: "wants to change but needs consolidation",
            });
        }
        out
    }

    pub fn proposals(&self) -> Vec<DriveProposal> {
        let mut drives = super::Drive::ALL
            .into_iter()
            .filter(|drive| self.get(*drive) >= 0.6)
            .map(|drive| (self.get(drive), proposal_for(drive)))
            .collect::<Vec<_>>();
        drives.sort_by(|left, right| right.0.total_cmp(&left.0));
        drives.into_iter().map(|(_, proposal)| proposal).collect()
    }
}

fn proposal_for(drive: super::Drive) -> DriveProposal {
    match drive {
        super::Drive::Connection => DriveProposal::CheckIn,
        super::Drive::Curiosity => DriveProposal::Explore,
        super::Drive::Mastery => DriveProposal::Build,
        super::Drive::Coherence => DriveProposal::Refactor,
        super::Drive::Rest => DriveProposal::Consolidate,
        super::Drive::Growth => DriveProposal::Evolve,
    }
}
