//! Identity use cases: diary, trait distillation, decay, and rendering.

use orexis_core::{
    render_identity, self_view as render_self_view, EntityEvent, EntitySnapshot, Trait,
};
use orexis_ports::{EntityAppend, PortError, PortResult};

pub async fn append_diary(
    append: &dyn EntityAppend,
    text: &str,
    source_seq: Option<u64>,
) -> PortResult<()> {
    if text.trim().is_empty() {
        return Err(PortError("diary text is empty".into()));
    }
    append
        .append(EntityEvent::DiaryAppend {
            text: text.to_string(),
            source_seq,
        })
        .await?;
    Ok(())
}

pub async fn distill_trait(
    append: &dyn EntityAppend,
    id: &str,
    statement: &str,
    evidence: Vec<u64>,
    confidence: f32,
) -> PortResult<()> {
    let trait_ = Trait::new(id, statement, evidence, confidence)
        .ok_or_else(|| PortError("trait needs a statement and evidence".into()))?;
    append
        .append(EntityEvent::TraitUpdate {
            trait_id: trait_.id,
            statement: trait_.statement,
            evidence: trait_.evidence,
            confidence: trait_.confidence,
        })
        .await?;
    Ok(())
}

pub async fn decay_traits(
    append: &dyn EntityAppend,
    snapshot: &EntitySnapshot,
    rate: f32,
) -> PortResult<()> {
    for trait_ in snapshot.traits.values() {
        let decayed = trait_.decayed(rate);
        if (decayed.confidence - trait_.confidence).abs() > f32::EPSILON {
            append
                .append(EntityEvent::TraitUpdate {
                    trait_id: decayed.id,
                    statement: decayed.statement,
                    evidence: decayed.evidence,
                    confidence: decayed.confidence,
                })
                .await?;
        }
    }
    Ok(())
}

pub fn identity(snapshot: &EntitySnapshot) -> String {
    render_identity(snapshot)
}

pub fn self_view(snapshot: &EntitySnapshot) -> String {
    render_self_view(snapshot)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    use async_trait::async_trait;
    use futures::executor::block_on;
    use orexis_core::Trait;
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

    #[test]
    fn diary_and_trait_append() {
        let append = RecordingAppend::default();
        block_on(append_diary(&append, "learned the build", Some(3))).expect("diary");
        block_on(distill_trait(
            &append,
            "t1",
            "prefers tests first",
            vec![3, 7],
            0.7,
        ))
        .expect("trait");
        let events = append.events.lock().expect("events");
        assert!(matches!(events[0], EntityEvent::DiaryAppend { .. }));
        assert!(matches!(events[1], EntityEvent::TraitUpdate { .. }));
    }

    #[test]
    fn traits_without_evidence_are_rejected() {
        let append = RecordingAppend::default();
        assert!(block_on(distill_trait(&append, "t1", "no evidence", vec![], 0.5)).is_err());
    }

    #[test]
    fn decay_appends_updated_confidence() {
        let mut snapshot = EntitySnapshot::default();
        snapshot.traits.insert(
            "t1".to_string(),
            Trait::new("t1", "x", vec![1], 0.8).expect("trait"),
        );
        let append = RecordingAppend::default();
        block_on(decay_traits(&append, &snapshot, 0.5)).expect("decay");
        let events = append.events.lock().expect("events");
        assert!(matches!(
            events.as_slice(),
            [EntityEvent::TraitUpdate { confidence, .. }] if (*confidence - 0.4).abs() < 0.001
        ));
    }
}
