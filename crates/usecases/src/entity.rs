//! Entity use cases: drive outcomes, decay, and boundary proposals.

use orexis_core::{Drive, DriveOutcome, DriveProposal, Drives, EntityEvent, EntitySnapshot};
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

pub fn proposals(snapshot: &EntitySnapshot) -> Vec<DriveProposal> {
    snapshot.drives.proposals()
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
}
