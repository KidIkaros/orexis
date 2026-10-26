//! Entity state types.

//! Entity domain types: drives, PAD affect, posture, traits, memory, and events.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub const ENTITY_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Drive {
    Connection,
    Curiosity,
    Mastery,
    Coherence,
    Rest,
    Growth,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Drives {
    pub connection: f32,
    pub curiosity: f32,
    pub mastery: f32,
    pub coherence: f32,
    pub rest: f32,
    pub growth: f32,
}

impl Default for Drives {
    fn default() -> Self {
        Self {
            connection: 0.5,
            curiosity: 0.5,
            mastery: 0.5,
            coherence: 0.5,
            rest: 0.5,
            growth: 0.5,
        }
    }
}

impl Drives {
    pub fn get(&self, drive: Drive) -> f32 {
        match drive {
            Drive::Connection => self.connection,
            Drive::Curiosity => self.curiosity,
            Drive::Mastery => self.mastery,
            Drive::Coherence => self.coherence,
            Drive::Rest => self.rest,
            Drive::Growth => self.growth,
        }
    }

    pub fn set(&mut self, drive: Drive, value: f32) {
        let value = value.clamp(0.0, 1.0);
        match drive {
            Drive::Connection => self.connection = value,
            Drive::Curiosity => self.curiosity = value,
            Drive::Mastery => self.mastery = value,
            Drive::Coherence => self.coherence = value,
            Drive::Rest => self.rest = value,
            Drive::Growth => self.growth = value,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Affect {
    pub valence: f32,
    pub arousal: f32,
    pub dominance: f32,
}

impl Default for Affect {
    fn default() -> Self {
        Self {
            valence: 0.0,
            arousal: 0.0,
            dominance: 0.0,
        }
    }
}

impl Affect {
    pub fn clamped(self) -> Self {
        Self {
            valence: self.valence.clamp(-1.0, 1.0),
            arousal: self.arousal.clamp(-1.0, 1.0),
            dominance: self.dominance.clamp(-1.0, 1.0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelTier {
    Fast,
    #[default]
    Standard,
    Deep,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Thinking {
    Off,
    #[default]
    Auto,
    On,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerifyLevel {
    #[default]
    OnRisk,
    Always,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Risk {
    Cautious,
    #[default]
    Normal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Exploration {
    Low,
    #[default]
    Normal,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Posture {
    pub model_tier: ModelTier,
    pub thinking: Thinking,
    pub repair_budget: u8,
    pub verify: VerifyLevel,
    pub risk: Risk,
    pub exploration: Exploration,
}

impl Default for Posture {
    fn default() -> Self {
        Self {
            model_tier: ModelTier::Standard,
            thinking: Thinking::Auto,
            repair_budget: 2,
            verify: VerifyLevel::OnRisk,
            risk: Risk::Normal,
            exploration: Exploration::Normal,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryKind {
    Episodic,
    Semantic,
    Procedural,
    Identity,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Trait {
    pub id: String,
    pub statement: String,
    pub evidence: Vec<u64>,
    pub confidence: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryRecord {
    pub id: String,
    pub kind: MemoryKind,
    pub text: String,
    pub tags: Vec<String>,
    pub source_seq: Option<u64>,
    pub confidence: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiaryEntry {
    pub text: String,
    pub source_seq: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EntitySnapshot {
    pub entity_id: Option<String>,
    pub drives: Drives,
    pub affect: Affect,
    pub posture: Posture,
    pub traits: BTreeMap<String, Trait>,
    pub memories: BTreeMap<String, MemoryRecord>,
    pub diary: Vec<DiaryEntry>,
}
