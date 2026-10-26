//! Entity use cases: drive outcomes, decay, and boundary proposals.

use orexis_core::{
    modulate_drives, Drive, DriveOutcome, DriveProposal, Drives, EntityEvent, EntitySnapshot,
    Posture,
};
use orexis_ports::{EntityAppend, PortResult};

pub async fn record_outcome(
    append: &dyn EntityAppend,
    snapshot: &EntitySnapshot,
    outcome: DriveOutcome,
) -> PortResult<()> {
    let updated = snapshot.drives.after(outcome);
    append_changes(
        append,
        &snapshot.drives,
        &updated,
        &format!("outcome: {outcome:?}"),
    )
    .await
}

pub async fn decay(
    append: &dyn EntityAppend,
    snapshot: &EntitySnapshot,
    rate: f32,
) -> PortResult<()> {
    let updated = snapshot.drives.decay(Drives::baseline(), rate);
    append_changes(append, &snapshot.drives, &updated, "decay").await
}

pub async fn record_affect(
    append: &dyn EntityAppend,
    snapshot: &EntitySnapshot,
    outcome: DriveOutcome,
) -> PortResult<()> {
    let affect = snapshot.affect.after(outcome);
    append
        .append(EntityEvent::AffectUpdate {
            valence: affect.valence,
            arousal: affect.arousal,
            dominance: affect.dominance,
            reason: format!("outcome: {outcome:?}"),
        })
        .await?;
    let posture = Posture::for_affect(affect);
    if posture != snapshot.posture {
        append
            .append(EntityEvent::PostureChange {
                posture,
                reason: format!("outcome: {outcome:?}"),
            })
            .await?;
    }
    Ok(())
}

pub fn posture(snapshot: &EntitySnapshot) -> Posture {
    snapshot.posture
}

pub fn modulated_drives(snapshot: &EntitySnapshot) -> Drives {
    modulate_drives(snapshot.drives, snapshot.affect)
}

pub fn proposals(snapshot: &EntitySnapshot) -> Vec<DriveProposal> {
    modulate_drives(snapshot.drives, snapshot.affect).proposals()
}

async fn append_changes(
    append: &dyn EntityAppend,
    before: &Drives,
    after: &Drives,
    reason: &str,
) -> PortResult<()> {
    for drive in Drive::ALL {
        let previous = before.get(drive);
        let value = after.get(drive);
        if (value - previous).abs() > f32::EPSILON {
            append
                .append(EntityEvent::DriveUpdate {
                    drive,
                    value,
                    reason: reason.to_string(),
                })
                .await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    use async_trait::async_trait;
    use futures::executor::block_on;
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

    fn snapshot() -> EntitySnapshot {
        let mut drives = Drives::default();
        drives.set(Drive::Mastery, 0.4);
        EntitySnapshot {
            drives,
            ..EntitySnapshot::default()
        }
    }

    #[test]
    fn outcome_appends_only_changed_drives() {
        let append = RecordingAppend::default();
        block_on(record_outcome(&append, &snapshot(), DriveOutcome::TaskDone)).expect("record");
        let events = append.events.lock().expect("events");
        assert!(!events.is_empty());
        assert!(events
            .iter()
            .all(|event| matches!(event, EntityEvent::DriveUpdate { .. })));
        assert!(events.iter().any(|event| matches!(
            event,
            EntityEvent::DriveUpdate {
                drive: Drive::Mastery,
                ..
            }
        )));
    }

    #[test]
    fn decay_appends_drive_updates() {
        let append = RecordingAppend::default();
        let mut snapshot = snapshot();
        snapshot.drives.set(Drive::Growth, 0.9);
        block_on(decay(&append, &snapshot, 0.5)).expect("decay");
        assert!(append
            .events
            .lock()
            .expect("events")
            .iter()
            .any(|event| matches!(
                event,
                EntityEvent::DriveUpdate {
                    drive: Drive::Growth,
                    ..
                }
            )));
    }

    #[test]
    fn proposals_are_boundary_only_and_ordered() {
        let mut snapshot = snapshot();
        snapshot.drives.set(Drive::Curiosity, 0.9);
        snapshot.drives.set(Drive::Mastery, 0.7);
        let proposals = proposals(&snapshot);
        assert_eq!(proposals[0], DriveProposal::Explore);
    }

    #[test]
    fn affect_updates_posture_after_failure() {
        let append = RecordingAppend::default();
        block_on(record_affect(
            &append,
            &snapshot(),
            DriveOutcome::TaskFailed,
        ))
        .expect("affect");
        let events = append.events.lock().expect("events");
        assert!(events
            .iter()
            .any(|event| matches!(event, EntityEvent::AffectUpdate { .. })));
        let posture = events.iter().find_map(|event| match event {
            EntityEvent::PostureChange { posture, .. } => Some(*posture),
            _ => None,
        });
        let posture = posture.expect("posture change");
        assert_eq!(posture.risk, orexis_core::Risk::Cautious);
        assert_eq!(posture.verify, orexis_core::VerifyLevel::Always);
    }

    #[test]
    fn glial_modulation_is_bounded() {
        let mut snapshot = snapshot();
        snapshot.affect = orexis_core::Affect {
            valence: -1.0,
            arousal: 1.0,
            dominance: -1.0,
        };
        let drives = modulated_drives(&snapshot);
        for drive in Drive::ALL {
            assert!((0.0..=1.0).contains(&drives.get(drive)));
        }
    }
}
