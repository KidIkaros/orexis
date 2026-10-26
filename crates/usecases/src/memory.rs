//! Memory use cases: remember, forget, and lexical recall.

use orexis_core::{
    recall as core_recall, EntityEvent, EntitySnapshot, MemoryQuery, MemoryRecord, ScoredMemory,
};
use orexis_ports::{EntityAppend, PortError, PortResult};

pub async fn remember(append: &dyn EntityAppend, memory: MemoryRecord) -> PortResult<()> {
    if memory.text.trim().is_empty() {
        return Err(PortError("memory text is empty".into()));
    }
    append
        .append(EntityEvent::MemoryWrite {
            memory_id: memory.id,
            kind: memory.kind,
            text: memory.text,
            tags: memory.tags,
            source_seq: memory.source_seq,
            confidence: memory.confidence.clamp(0.0, 1.0),
        })
        .await?;
    Ok(())
}

pub async fn forget(append: &dyn EntityAppend, memory_id: &str) -> PortResult<()> {
    append
        .append(EntityEvent::MemoryForget {
            memory_id: memory_id.to_string(),
        })
        .await?;
    Ok(())
}

pub fn recall(snapshot: &EntitySnapshot, query: &MemoryQuery) -> Vec<ScoredMemory> {
    core_recall(&snapshot.memories, query)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    use async_trait::async_trait;
    use futures::executor::block_on;
    use orexis_core::MemoryKind;
    use orexis_ports::PortResult;

    #[derive(Default)]
    struct RecordingAppend {
        events: Mutex<Vec<EntityEvent>>,
    }

    #[async_trait]
    impl EntityAppend for RecordingAppend {
        async fn append(&self, event: EntityEvent) -> PortResult<u64> {
            let mut events = self.events.lock().expect("events");
            events.push(event);
            Ok(events.len() as u64)
        }
    }

    fn memory() -> MemoryRecord {
        MemoryRecord {
            id: "m1".to_string(),
            kind: MemoryKind::Semantic,
            text: "repo uses cargo".to_string(),
            tags: vec!["repo".to_string()],
            source_seq: Some(3),
            confidence: 0.8,
        }
    }

    #[test]
    fn remember_appends_a_write() {
        let append = RecordingAppend::default();
        block_on(remember(&append, memory())).expect("remember");
        assert!(matches!(
            append.events.lock().expect("events").as_slice(),
            [EntityEvent::MemoryWrite { .. }]
        ));
    }

    #[test]
    fn forget_appends_a_forget() {
        let append = RecordingAppend::default();
        block_on(forget(&append, "m1")).expect("forget");
        assert!(matches!(
            append.events.lock().expect("events").as_slice(),
            [EntityEvent::MemoryForget { .. }]
        ));
    }

    #[test]
    fn recall_uses_the_snapshot() {
        let mut snapshot = EntitySnapshot::default();
        snapshot.memories.insert("m1".to_string(), memory());
        let results = recall(
            &snapshot,
            &MemoryQuery {
                text: "cargo".to_string(),
                ..MemoryQuery::default()
            },
        );
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].record.id, "m1");
    }
}
