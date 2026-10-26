//! Entity events and redaction.

use serde::{Deserialize, Serialize};

use crate::{redact_text, Drive, MemoryKind, Posture};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum EntityEvent {
    EntityStart {
        entity_id: String,
        format_version: u32,
    },
    DriveUpdate {
        drive: Drive,
        value: f32,
        reason: String,
    },
    AffectUpdate {
        valence: f32,
        arousal: f32,
        dominance: f32,
        reason: String,
    },
    PostureChange {
        posture: Posture,
        reason: String,
    },
    DiaryAppend {
        text: String,
        source_seq: Option<u64>,
    },
    TraitUpdate {
        trait_id: String,
        statement: String,
        evidence: Vec<u64>,
        confidence: f32,
    },
    MemoryWrite {
        memory_id: String,
        kind: MemoryKind,
        text: String,
        tags: Vec<String>,
        source_seq: Option<u64>,
        confidence: f32,
    },
    MemoryForget {
        memory_id: String,
    },
}

impl EntityEvent {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::EntityStart { .. } => "entity_start",
            Self::DriveUpdate { .. } => "drive_update",
            Self::AffectUpdate { .. } => "affect_update",
            Self::PostureChange { .. } => "posture_change",
            Self::DiaryAppend { .. } => "diary_append",
            Self::TraitUpdate { .. } => "trait_update",
            Self::MemoryWrite { .. } => "memory_write",
            Self::MemoryForget { .. } => "memory_forget",
        }
    }
}

pub fn redact_entity_event(event: &EntityEvent) -> EntityEvent {
    match event {
        EntityEvent::EntityStart {
            entity_id,
            format_version,
        } => EntityEvent::EntityStart {
            entity_id: entity_id.clone(),
            format_version: *format_version,
        },
        EntityEvent::DriveUpdate {
            drive,
            value,
            reason,
        } => EntityEvent::DriveUpdate {
            drive: *drive,
            value: *value,
            reason: redact_text(reason),
        },
        EntityEvent::AffectUpdate {
            valence,
            arousal,
            dominance,
            reason,
        } => EntityEvent::AffectUpdate {
            valence: *valence,
            arousal: *arousal,
            dominance: *dominance,
            reason: redact_text(reason),
        },
        EntityEvent::PostureChange { posture, reason } => EntityEvent::PostureChange {
            posture: *posture,
            reason: redact_text(reason),
        },
        EntityEvent::DiaryAppend { text, source_seq } => EntityEvent::DiaryAppend {
            text: redact_text(text),
            source_seq: *source_seq,
        },
        EntityEvent::TraitUpdate {
            trait_id,
            statement,
            evidence,
            confidence,
        } => EntityEvent::TraitUpdate {
            trait_id: trait_id.clone(),
            statement: redact_text(statement),
            evidence: evidence.clone(),
            confidence: *confidence,
        },
        EntityEvent::MemoryWrite {
            memory_id,
            kind,
            text,
            tags,
            source_seq,
            confidence,
        } => EntityEvent::MemoryWrite {
            memory_id: memory_id.clone(),
            kind: *kind,
            text: redact_text(text),
            tags: tags.iter().map(|tag| redact_text(tag)).collect(),
            source_seq: *source_seq,
            confidence: *confidence,
        },
        EntityEvent::MemoryForget { memory_id } => EntityEvent::MemoryForget {
            memory_id: memory_id.clone(),
        },
    }
}
