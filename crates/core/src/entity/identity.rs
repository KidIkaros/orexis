//! Identity distillation: evidence-backed traits, diary rendering, and `/self`.

use super::{EntitySnapshot, Trait};

impl Trait {
    pub fn new(
        id: impl Into<String>,
        statement: impl Into<String>,
        evidence: Vec<u64>,
        confidence: f32,
    ) -> Option<Self> {
        let statement = statement.into();
        if statement.trim().is_empty() || evidence.is_empty() {
            return None;
        }
        Some(Self {
            id: id.into(),
            statement,
            evidence,
            confidence: confidence.clamp(0.0, 1.0),
            last_seen_at: 0,
        })
    }

    pub fn seen(mut self, at: u64) -> Self {
        self.last_seen_at = at;
        self
    }

    pub fn decayed(&self, rate: f32) -> Self {
        let rate = rate.clamp(0.0, 1.0);
        Self {
            confidence: (self.confidence * (1.0 - rate)).clamp(0.0, 1.0),
            ..self.clone()
        }
    }
}

pub fn render_identity(snapshot: &EntitySnapshot) -> String {
    let mut traits = snapshot.traits.values().collect::<Vec<_>>();
    traits.sort_by(|left, right| {
        right
            .confidence
            .total_cmp(&left.confidence)
            .then(left.id.cmp(&right.id))
    });
    let mut out = String::from(
        "# Identity
",
    );
    if !traits.is_empty() {
        out.push_str("\n## Traits\n");
        for item in traits.iter().take(20) {
            out.push_str(&format!(
                "- {} (confidence {:.2}; evidence: {})\n",
                item.statement,
                item.confidence,
                item.evidence
                    .iter()
                    .map(u64::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }
    let diary = snapshot.diary.iter().rev().take(20).collect::<Vec<_>>();
    if !diary.is_empty() {
        out.push_str("\n## Recent diary\n");
        for entry in diary.into_iter().rev() {
            out.push_str(&format!("- {}\n", entry.text));
        }
    }
    out
}

pub fn identity_context(snapshot: &EntitySnapshot, max_chars: usize) -> String {
    let text = render_identity(snapshot);
    if snapshot.traits.is_empty() && snapshot.diary.is_empty() {
        return String::new();
    }
    text.chars().take(max_chars.max(1)).collect()
}

pub fn self_view(snapshot: &EntitySnapshot) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "drives   connection {:.2} · curiosity {:.2} · mastery {:.2} · coherence {:.2} · rest {:.2} · growth {:.2}\n",
        snapshot.drives.connection,
        snapshot.drives.curiosity,
        snapshot.drives.mastery,
        snapshot.drives.coherence,
        snapshot.drives.rest,
        snapshot.drives.growth,
    ));
    out.push_str(&format!(
        "affect   valence {:+.2} · arousal {:+.2} · dominance {:+.2} · mood {}\n",
        snapshot.affect.valence,
        snapshot.affect.arousal,
        snapshot.affect.dominance,
        snapshot.affect.mood().label(),
    ));
    out.push_str(&format!(
        "posture  {:?}/{:?} · repair {} · verify {:?} · risk {:?}\n",
        snapshot.posture.model_tier,
        snapshot.posture.thinking,
        snapshot.posture.repair_budget,
        snapshot.posture.verify,
        snapshot.posture.risk,
    ));
    out.push_str(&format!("traits   {}\n", snapshot.traits.len()));
    let mut traits = snapshot.traits.values().collect::<Vec<_>>();
    traits.sort_by(|left, right| right.confidence.total_cmp(&left.confidence));
    for item in traits.iter().take(10) {
        out.push_str(&format!(
            "  - {} ({:.2}; evidence {}; last seen {})\n",
            item.statement,
            item.confidence,
            item.evidence
                .iter()
                .map(u64::to_string)
                .collect::<Vec<_>>()
                .join(","),
            item.last_seen_at
        ));
    }
    out.push_str(&format!("memory   {}\n", snapshot.memories.len()));
    let diary = snapshot.diary.iter().rev().take(5).collect::<Vec<_>>();
    for entry in diary.into_iter().rev() {
        out.push_str(&format!("diary    {}\n", entry.text));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{DiaryEntry, EntitySnapshot, Trait};

    fn snapshot() -> EntitySnapshot {
        let mut snapshot = EntitySnapshot::default();
        snapshot.traits.insert(
            "t1".to_string(),
            Trait::new("t1", "prefers tests first", vec![3, 7], 0.7).expect("trait"),
        );
        snapshot.diary.push(DiaryEntry {
            text: "fixed the build".to_string(),
            source_seq: Some(3),
        });
        snapshot
    }

    #[test]
    fn traits_require_evidence() {
        assert!(Trait::new("t", "no evidence", vec![], 0.5).is_none());
        assert!(Trait::new("t", " ", vec![1], 0.5).is_none());
    }

    #[test]
    fn decay_reduces_confidence() {
        let trait_ = Trait::new("t", "x", vec![1], 0.8).expect("trait");
        assert!((trait_.decayed(0.5).confidence - 0.4).abs() < 0.001);
    }

    #[test]
    fn identity_and_self_render() {
        let snapshot = snapshot();
        let identity = render_identity(&snapshot);
        assert!(identity.contains("prefers tests first"));
        assert!(identity.contains("fixed the build"));
        let self_view = self_view(&snapshot);
        assert!(self_view.contains("drives"));
        assert!(self_view.contains("mood"));
        assert!(self_view.contains("traits   1"));
    }

    #[test]
    fn identity_context_is_bounded_and_empty_without_content() {
        let snapshot = snapshot();
        assert!(identity_context(&snapshot, 10).chars().count() <= 10);
        assert!(identity_context(&EntitySnapshot::default(), 100).is_empty());
    }
}
