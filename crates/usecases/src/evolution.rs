//! Evolution use cases: growth-gated proposals at session boundaries.

use orexis_core::{EntitySnapshot, Mutation};
use orexis_ports::{MutationQueue, PortError, PortResult, Proposal, ProposalContext};

pub const GROWTH_THRESHOLD: f32 = 0.6;

pub fn growth_allows(snapshot: &EntitySnapshot) -> bool {
    snapshot.drives.growth >= GROWTH_THRESHOLD
}

pub async fn propose_growth(
    queue: &dyn MutationQueue,
    snapshot: &EntitySnapshot,
    mutation: Mutation,
    context: ProposalContext,
    source: &str,
) -> PortResult<Proposal> {
    if !context.boundary {
        return Err(PortError("proposals are session-boundary only".into()));
    }
    if !growth_allows(snapshot) {
        return Err(PortError(
            "growth drive is below the proposal threshold".into(),
        ));
    }
    queue.propose(mutation, context, source).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    use async_trait::async_trait;
    use futures::executor::block_on;
    use orexis_core::{CandidateManifest, Drives, MutationKind};
    use orexis_ports::PortResult;

    #[derive(Default)]
    struct FakeQueue {
        proposals: Mutex<Vec<Mutation>>,
    }

    #[async_trait]
    impl MutationQueue for FakeQueue {
        async fn propose(
            &self,
            mutation: Mutation,
            context: ProposalContext,
            source: &str,
        ) -> PortResult<Proposal> {
            self.proposals
                .lock()
                .expect("proposals")
                .push(mutation.clone());
            let content_hash = mutation.hash();
            let size = mutation.size();
            Ok(Proposal {
                manifest: CandidateManifest {
                    id: "c1".to_string(),
                    kind: mutation.kind,
                    target: mutation.target,
                    base_hash: mutation.base_hash,
                    content_hash,
                    size,
                    session_id: context.session_id,
                    day: context.day,
                    created_at: 1,
                    source: source.to_string(),
                },
                deduplicated: false,
            })
        }

        async fn list(&self) -> PortResult<Vec<CandidateManifest>> {
            Ok(Vec::new())
        }

        async fn get(&self, _id: &str) -> PortResult<Option<CandidateManifest>> {
            Ok(None)
        }
    }

    fn snapshot(growth: f32) -> EntitySnapshot {
        let mut drives = Drives::default();
        drives.set(orexis_core::Drive::Growth, growth);
        EntitySnapshot {
            drives,
            ..EntitySnapshot::default()
        }
    }

    fn mutation() -> Mutation {
        Mutation {
            kind: MutationKind::GeneParameter,
            target: "drives.curiosity".to_string(),
            base_hash: "base".to_string(),
            content: "0.6".to_string(),
        }
    }

    fn context(boundary: bool) -> ProposalContext {
        ProposalContext {
            session_id: "s1".to_string(),
            day: 1,
            boundary,
        }
    }

    #[test]
    fn low_growth_rejects_and_high_growth_proposes() {
        let queue = FakeQueue::default();
        assert!(block_on(propose_growth(
            &queue,
            &snapshot(0.2),
            mutation(),
            context(true),
            "growth"
        ))
        .is_err());
        let proposal = block_on(propose_growth(
            &queue,
            &snapshot(0.9),
            mutation(),
            context(true),
            "growth",
        ))
        .expect("proposal");
        assert_eq!(proposal.manifest.id, "c1");
    }

    #[test]
    fn boundary_is_required() {
        let queue = FakeQueue::default();
        assert!(block_on(propose_growth(
            &queue,
            &snapshot(0.9),
            mutation(),
            context(false),
            "growth"
        ))
        .is_err());
    }
}
