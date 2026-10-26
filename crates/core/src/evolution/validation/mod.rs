//! Pure validation ladder for evolution candidates.

use serde::{Deserialize, Serialize};

use super::{Candidate, Seal};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationStage {
    Schema,
    Seal,
    StaticSafety,
    Deterministic,
    Behavioral,
    Soak,
    Human,
}

impl ValidationStage {
    pub fn label(self) -> &'static str {
        match self {
            Self::Schema => "schema",
            Self::Seal => "seal",
            Self::StaticSafety => "static_safety",
            Self::Deterministic => "deterministic",
            Self::Behavioral => "behavioral",
            Self::Soak => "soak",
            Self::Human => "human",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageResult {
    pub stage: ValidationStage,
    pub passed: bool,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationReport {
    pub candidate_id: String,
    pub stages: Vec<StageResult>,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeterministicTest {
    pub name: String,
    pub passed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BehavioralResult {
    pub baseline: f64,
    pub candidate: f64,
}

impl BehavioralResult {
    pub fn delta(self) -> f64 {
        self.candidate - self.baseline
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidationPolicy {
    pub noise_band: f64,
    pub soak_hours: u64,
    pub require_human: bool,
    pub require_deterministic_tests: bool,
}

impl Default for ValidationPolicy {
    fn default() -> Self {
        Self {
            noise_band: 0.05,
            soak_hours: 24,
            require_human: true,
            require_deterministic_tests: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidationInputs {
    pub candidate: Candidate,
    pub candidate_seal: Seal,
    pub current_seal: Seal,
    pub deterministic: Vec<DeterministicTest>,
    pub behavior: BehavioralResult,
    pub soak_hours: u64,
    pub human_approved: bool,
}

const SEALED_TARGETS: &[&str] = &[
    "permissions",
    "sandbox",
    "provider",
    "model",
    "tools",
    "codec",
    "protocol",
    "secrets",
    "telemetry",
    "safety",
    "identity",
];

pub fn validate(inputs: &ValidationInputs, policy: &ValidationPolicy) -> ValidationReport {
    let mut stages = Vec::new();

    let schema = inputs.candidate.mutation.validate();
    stages.push(StageResult {
        stage: ValidationStage::Schema,
        passed: schema.is_ok(),
        detail: schema.err().unwrap_or_else(|| "schema ok".to_string()),
    });

    let seal_ok = inputs.candidate_seal.hash == inputs.current_seal.hash;
    stages.push(StageResult {
        stage: ValidationStage::Seal,
        passed: seal_ok,
        detail: if seal_ok {
            "seal unchanged".to_string()
        } else {
            "sealed configuration changed".to_string()
        },
    });

    let root = inputs
        .candidate
        .mutation
        .target
        .split('.')
        .next()
        .unwrap_or_default();
    let static_ok = !SEALED_TARGETS.contains(&root);
    stages.push(StageResult {
        stage: ValidationStage::StaticSafety,
        passed: static_ok,
        detail: if static_ok {
            "target is mutable data".to_string()
        } else {
            format!("target {root} is sealed")
        },
    });

    let deterministic_ok = if inputs.deterministic.is_empty() {
        !policy.require_deterministic_tests
    } else {
        inputs.deterministic.iter().all(|test| test.passed)
    };
    stages.push(StageResult {
        stage: ValidationStage::Deterministic,
        passed: deterministic_ok,
        detail: format!(
            "{} tests; {} failed",
            inputs.deterministic.len(),
            inputs
                .deterministic
                .iter()
                .filter(|test| !test.passed)
                .count()
        ),
    });

    let behavioral_ok = inputs.behavior.delta() >= -policy.noise_band;
    stages.push(StageResult {
        stage: ValidationStage::Behavioral,
        passed: behavioral_ok,
        detail: format!(
            "delta {:+.3} vs noise {:.3}",
            inputs.behavior.delta(),
            policy.noise_band
        ),
    });

    let soak_ok = inputs.soak_hours >= policy.soak_hours;
    stages.push(StageResult {
        stage: ValidationStage::Soak,
        passed: soak_ok,
        detail: format!("{}h of {}h", inputs.soak_hours, policy.soak_hours),
    });

    let human_ok = !policy.require_human || inputs.human_approved;
    stages.push(StageResult {
        stage: ValidationStage::Human,
        passed: human_ok,
        detail: if human_ok {
            "human approved or not required".to_string()
        } else {
            "human approval pending".to_string()
        },
    });

    let passed = stages.iter().all(|stage| stage.passed);
    ValidationReport {
        candidate_id: inputs.candidate.manifest.id.clone(),
        stages,
        passed,
    }
}

#[cfg(test)]
mod tests;
