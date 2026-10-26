//! Entity store, projection, restart, export/import, and redaction tests.

use futures::executor::block_on;
use orexis_adapters_entity_sqlite::{import_jsonl, SqliteEntityStore};
use orexis_core::{Drive, EntityEvent, MemoryKind};
use orexis_ports::{EntityAppend, EntityRead};

#[test]
fn append_and_project() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = SqliteEntityStore::open(&dir.path().join("entity.db")).expect("store");
    block_on(store.append(EntityEvent::DriveUpdate {
        drive: Drive::Growth,
        value: 0.8,
        reason: "pattern".to_string(),
    }))
    .expect("drive");
    block_on(store.append(EntityEvent::AffectUpdate {
        valence: 0.3,
        arousal: 0.6,
        dominance: -0.1,
        reason: "failure".to_string(),
    }))
    .expect("affect");
    block_on(store.append(EntityEvent::MemoryWrite {
        memory_id: "m1".to_string(),
        kind: MemoryKind::Semantic,
        text: "repo uses cargo".to_string(),
        tags: vec!["repo".to_string()],
        source_seq: Some(2),
        confidence: 0.8,
    }))
    .expect("memory");
    let snapshot = block_on(store.snapshot()).expect("snapshot");
    assert_eq!(snapshot.drives.growth, 0.8);
    assert_eq!(snapshot.affect.arousal, 0.6);
    assert_eq!(snapshot.memories["m1"].kind, MemoryKind::Semantic);
    assert_eq!(snapshot.entity_id.as_deref(), Some(store.entity_id()));
}

#[test]
fn restart_preserves_the_log() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("entity.db");
    {
        let store = SqliteEntityStore::open(&path).expect("store");
        block_on(store.append(EntityEvent::DiaryAppend {
            text: "first".to_string(),
            source_seq: None,
        }))
        .expect("diary");
    }
    let reopened = SqliteEntityStore::open(&path).expect("reopen");
    let events = block_on(reopened.events()).expect("events");
    assert!(events.iter().any(
        |stored| matches!(&stored.event, EntityEvent::DiaryAppend { text, .. } if text == "first")
    ));
}

#[test]
fn export_import_round_trips() {
    let dir = tempfile::tempdir().expect("tempdir");
    let source = SqliteEntityStore::open(&dir.path().join("a.db")).expect("source");
    block_on(source.append(EntityEvent::AffectUpdate {
        valence: -0.4,
        arousal: 0.5,
        dominance: -0.2,
        reason: "verify failed".to_string(),
    }))
    .expect("affect");
    let jsonl = source.export_jsonl().expect("export");
    let target = dir.path().join("b.db");
    import_jsonl(&target, &jsonl).expect("import");
    let imported = SqliteEntityStore::open(&target).expect("target");
    assert_eq!(
        block_on(source.events()).expect("source events"),
        block_on(imported.events()).expect("target events")
    );
}

#[test]
fn redacts_at_write() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = SqliteEntityStore::open(&dir.path().join("entity.db")).expect("store");
    block_on(store.append(EntityEvent::DiaryAppend {
        text: "use sk-abcdefghijklmnopqrstuvwx".to_string(),
        source_seq: None,
    }))
    .expect("diary");
    let snapshot = block_on(store.snapshot()).expect("snapshot");
    let text = &snapshot.diary[0].text;
    assert!(text.contains("[REDACTED:"));
    assert!(!text.contains("sk-abcdefghijklmnopqrstuvwx"));
}
