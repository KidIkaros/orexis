//! Pure projection from entity events to current state.

use super::{Affect, DiaryEntry, EntityEvent, EntitySnapshot, MemoryRecord, Trait};

pub fn project(events: &[EntityEvent]) -> EntitySnapshot {
    let mut snapshot = EntitySnapshot::default();
    for event in events {
        match event {
            EntityEvent::EntityStart { entity_id, .. } => {
                snapshot.entity_id = Some(entity_id.clone());
            }
            EntityEvent::DriveUpdate { drive, value, .. } => snapshot.drives.set(*drive, *value),
            EntityEvent::AffectUpdate {
                valence,
                arousal,
                dominance,
                ..
            } => {
                snapshot.affect = Affect {
                    valence: *valence,
                    arousal: *arousal,
                    dominance: *dominance,
                }
                .clamped();
            }
            EntityEvent::PostureChange { posture, .. } => snapshot.posture = *posture,
            EntityEvent::DiaryAppend { text, source_seq } => {
                snapshot.diary.push(DiaryEntry {
                    text: text.clone(),
                    source_seq: *source_seq,
                });
            }
            EntityEvent::TraitUpdate {
                trait_id,
                statement,
                evidence,
                confidence,
            } => {
                snapshot.traits.insert(
                    trait_id.clone(),
                    Trait {
                        id: trait_id.clone(),
                        statement: statement.clone(),
                        evidence: evidence.clone(),
                        confidence: *confidence,
                    },
                );
            }
            EntityEvent::MemoryWrite {
                memory_id,
                kind,
                text,
                tags,
                source_seq,
                confidence,
            } => {
                snapshot.memories.insert(
                    memory_id.clone(),
                    MemoryRecord {
                        id: memory_id.clone(),
                        kind: *kind,
                        text: text.clone(),
                        tags: tags.clone(),
                        source_seq: *source_seq,
                        confidence: *confidence,
                    },
                );
            }
            EntityEvent::MemoryForget { memory_id } => {
                snapshot.memories.remove(memory_id);
            }
        }
    }
    snapshot
}
