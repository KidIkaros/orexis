//! Apply and rollback evolution candidates through the tier policy.

use orexis_core::{CandidateRef, LedgerEntry, RollbackTrigger, Seal, Tier, ValidationReport};
use orexis_ports::{EvolutionStore, MutationQueue, PortError, PortResult};

pub struct ApplyRequest {
    pub candidate_id: String,
    pub tier: Tier,
    pub human_approved: bool,
    pub signed: bool,
    pub validation: ValidationReport,
    pub seal: Seal,
}

pub async fn apply_candidate(
    queue: &dyn MutationQueue,
    corridor: &dyn EvolutionStore,
    request: ApplyRequest,
) -> PortResult<LedgerEntry> {
    if !request.validation.passed {
        return Err(PortError("validation did not pass".into()));
    }
    if request.validation.candidate_id != request.candidate_id {
        return Err(PortError("validation is for a different candidate".into()));
    }
    let candidate = queue
        .candidate(&request.candidate_id)
        .await?
        .ok_or_else(|| PortError(format!("unknown candidate {}", request.candidate_id)))?;
    if !request.tier.allows_apply(
        candidate.mutation.kind,
        request.human_approved,
        request.signed,
    ) {
        return Err(PortError(format!(
            "tier {:?} does not allow applying {:?}",
            request.tier, candidate.mutation.kind
        )));
    }
    corridor
        .apply(
            CandidateRef {
                id: candidate.manifest.id,
                content_hash: candidate.manifest.content_hash,
            },
            request.seal,
        )
        .await
}

pub async fn rollback(
    corridor: &dyn EvolutionStore,
    candidate_id: &str,
    reason: &str,
) -> PortResult<LedgerEntry> {
    corridor.revert(candidate_id, reason).await
}

pub fn should_rollback(tier: Tier, trigger: RollbackTrigger) -> bool {
    tier.auto_rollback(trigger)
}

#[cfg(test)]
mod tests;
