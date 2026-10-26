use super::*;

#[test]
fn projection_applies_entity_events() {
    let events = vec![
        EntityEvent::EntityStart {
            entity_id: "e1".to_string(),
            format_version: ENTITY_FORMAT_VERSION,
        },
        EntityEvent::DriveUpdate {
            drive: Drive::Curiosity,
            value: 0.8,
            reason: "unknown code".to_string(),
        },
        EntityEvent::AffectUpdate {
            valence: 0.4,
            arousal: 0.7,
            dominance: -0.2,
            reason: "failure".to_string(),
        },
        EntityEvent::DiaryAppend {
            text: "learned the build".to_string(),
            source_seq: Some(3),
        },
        EntityEvent::TraitUpdate {
            trait_id: "t1".to_string(),
            statement: "prefers tests first".to_string(),
            evidence: vec![3, 7],
            confidence: 0.7,
        },
        EntityEvent::MemoryWrite {
            memory_id: "m1".to_string(),
            kind: MemoryKind::Semantic,
            text: "repo uses cargo".to_string(),
            tags: vec!["repo".to_string()],
            source_seq: Some(4),
            confidence: 0.8,
        },
    ];
    let snapshot = project(&events);
    assert_eq!(snapshot.entity_id.as_deref(), Some("e1"));
    assert_eq!(snapshot.drives.curiosity, 0.8);
    assert_eq!(snapshot.affect.valence, 0.4);
    assert_eq!(snapshot.traits["t1"].evidence, vec![3, 7]);
    assert_eq!(snapshot.memories["m1"].kind, MemoryKind::Semantic);
    assert_eq!(snapshot.diary.len(), 1);
}

#[test]
fn drives_clamp_and_redaction_applies() {
    let mut drives = Drives::default();
    drives.set(Drive::Growth, 2.0);
    assert_eq!(drives.growth, 1.0);
    let event = EntityEvent::DiaryAppend {
        text: "key sk-abcdefghijklmnopqrstuvwx".to_string(),
        source_seq: None,
    };
    let redacted = redact_entity_event(&event);
    let EntityEvent::DiaryAppend { text, .. } = redacted else {
        panic!("wrong variant");
    };
    assert!(text.contains("[REDACTED:"));
    assert!(!text.contains("sk-abcdefghijklmnopqrstuvwx"));
}
