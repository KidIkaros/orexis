//! Store, fork, redaction, export, and migration tests.

use futures::executor::block_on;
use orexis_adapters_session_sqlite::SqliteSessionStore;
use orexis_core::SessionEvent;
use orexis_ports::{SessionAppend, SessionRead};
use rusqlite::{params, Connection};

fn user(text: &str) -> SessionEvent {
    SessionEvent::UserMessage {
        text: text.to_string(),
    }
}

#[test]
fn append_and_read_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let store = SqliteSessionStore::open(&dir.path().join("s.db")).unwrap();
    block_on(store.append(user("one"))).unwrap();
    block_on(store.append(user("two"))).unwrap();

    let events = block_on(store.events()).unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].seq, 1);
    assert_eq!(events[1].seq, 2);
    assert_eq!(events[0].generation_id, 1);
    assert!(events[0].event_id.len() > 10);
}

#[test]
fn reopen_preserves_events() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("s.db");
    let id = {
        let store = SqliteSessionStore::open(&path).unwrap();
        block_on(store.append(user("one"))).unwrap();
        store.session_id().to_string()
    };
    let reopened = SqliteSessionStore::open_session(&path, &id).unwrap();
    let events = block_on(reopened.events()).unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].seq, 1);
}

#[test]
fn fork_copies_the_prefix() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("s.db");
    let store = SqliteSessionStore::open(&path).unwrap();
    for text in ["one", "two", "three"] {
        block_on(store.append(user(text))).unwrap();
    }
    let forked = store.fork_at(2).unwrap();
    assert_ne!(forked, store.session_id());

    let child = SqliteSessionStore::open_session(&path, &forked).unwrap();
    let events = block_on(child.events()).unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[1].seq, 2);

    let parent_events = block_on(store.events()).unwrap();
    assert_eq!(parent_events.len(), 3);
}

#[test]
fn redacts_at_write() {
    let dir = tempfile::tempdir().unwrap();
    let store = SqliteSessionStore::open(&dir.path().join("s.db")).unwrap();
    block_on(store.append(user("use sk-abcdefghijklmnopqrstuvwx now"))).unwrap();
    let events = block_on(store.events()).unwrap();
    let SessionEvent::UserMessage { text } = &events[0].event else {
        panic!("wrong variant");
    };
    assert!(text.contains("[REDACTED:"));
    assert!(!text.contains("sk-abcdefghijklmnopqrstuvwx"));
}

#[test]
fn export_contains_every_event() {
    let dir = tempfile::tempdir().unwrap();
    let store = SqliteSessionStore::open(&dir.path().join("s.db")).unwrap();
    block_on(store.append(user("one"))).unwrap();
    let jsonl = store.export_jsonl().unwrap();
    assert!(jsonl.contains("\"seq\":1"));
    assert!(jsonl.contains("user_message"));
}

#[test]
fn migrates_a_v1_event_table() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("v1.db");
    {
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(
            "CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
                 INSERT INTO meta VALUES ('schema_version', '1');
                 CREATE TABLE session (
                    id TEXT PRIMARY KEY, parent_id TEXT, fork_seq INTEGER,
                    format_version INTEGER NOT NULL, profile_id TEXT, model_id TEXT,
                    created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
                 );
                 CREATE TABLE event (
                    session_id TEXT NOT NULL, seq INTEGER NOT NULL, event_id TEXT NOT NULL,
                    kind TEXT NOT NULL, version INTEGER NOT NULL,
                    payload_json TEXT NOT NULL, created_at INTEGER NOT NULL,
                    PRIMARY KEY (session_id, seq)
                 );
                 CREATE TABLE generation (
                    session_id TEXT NOT NULL, generation_id INTEGER NOT NULL,
                    starts_at_seq INTEGER NOT NULL, reason TEXT NOT NULL,
                    PRIMARY KEY (session_id, generation_id)
                 );
                 INSERT INTO session VALUES ('s1', NULL, NULL, 1, NULL, NULL, 0, 0);
                 INSERT INTO generation VALUES ('s1', 1, 1, 'start');",
        )
        .unwrap();
        let payload = serde_json::to_string(&user("old")).unwrap();
        conn.execute(
            "INSERT INTO event (session_id, seq, event_id, kind, version, payload_json, created_at)
                 VALUES ('s1', 1, 'e1', 'user_message', 1, ?1, 0)",
            params![payload],
        )
        .unwrap();
    }

    let store = SqliteSessionStore::open_session(&path, "s1").unwrap();
    let events = block_on(store.events()).unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].generation_id, 1);
}

#[test]
fn export_all_and_import_all_round_trip_without_loss() {
    let dir = tempfile::tempdir().unwrap();
    let src_path = dir.path().join("src.db");
    let s1 = SqliteSessionStore::open(&src_path).unwrap();
    let s1_id = s1.session_id().to_string();
    block_on(s1.append(user("alpha"))).unwrap();
    block_on(s1.append(user("beta"))).unwrap();
    let s2_id = s1.fork_at(1).unwrap();
    let s2 = SqliteSessionStore::open_session(&src_path, &s2_id).unwrap();
    block_on(s2.append(user("gamma"))).unwrap();

    let jsonl = SqliteSessionStore::export_all_jsonl(&src_path).unwrap();
    let dst_path = dir.path().join("dst.db");
    let imported = SqliteSessionStore::import_all_jsonl(&dst_path, &jsonl).unwrap();
    assert_eq!(imported, 4); // 2 in s1 + 1 copied in s2 + 1 appended in s2

    let dst_sessions = SqliteSessionStore::list_sessions(&dst_path).unwrap();
    assert_eq!(dst_sessions, vec![s1_id.clone(), s2_id.clone()]);

    let dst_s1 = SqliteSessionStore::open_session(&dst_path, &s1_id).unwrap();
    let dst_s2 = SqliteSessionStore::open_session(&dst_path, &s2_id).unwrap();
    assert_eq!(
        block_on(s1.events()).unwrap(),
        block_on(dst_s1.events()).unwrap()
    );
    assert_eq!(
        block_on(s2.events()).unwrap(),
        block_on(dst_s2.events()).unwrap()
    );
}
