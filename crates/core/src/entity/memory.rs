//! Lexical memory recall over entity memory records.

use std::collections::BTreeMap;

use super::{MemoryKind, MemoryRecord};

#[derive(Debug, Clone, PartialEq)]
pub struct MemoryQuery {
    pub text: String,
    pub tags: Vec<String>,
    pub kinds: Vec<MemoryKind>,
    pub limit: usize,
}

impl Default for MemoryQuery {
    fn default() -> Self {
        Self {
            text: String::new(),
            tags: Vec::new(),
            kinds: Vec::new(),
            limit: 10,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScoredMemory {
    pub record: MemoryRecord,
    pub score: f32,
}

pub fn recall(memories: &BTreeMap<String, MemoryRecord>, query: &MemoryQuery) -> Vec<ScoredMemory> {
    let tokens = tokenize(&query.text);
    let tags = query
        .tags
        .iter()
        .map(|tag| tag.to_lowercase())
        .collect::<Vec<_>>();
    let has_query = !tokens.is_empty() || !tags.is_empty();
    let limit = query.limit.clamp(1, 50);
    let mut scored = Vec::new();

    for record in memories.values() {
        if !query.kinds.is_empty() && !query.kinds.contains(&record.kind) {
            continue;
        }
        let text = record.text.to_lowercase();
        let record_tags = record
            .tags
            .iter()
            .map(|tag| tag.to_lowercase())
            .collect::<Vec<_>>();
        let mut score = record.confidence.clamp(0.0, 1.0) * 0.5;
        let mut matched = false;
        for token in &tokens {
            if text.contains(token) {
                score += 1.0;
                matched = true;
            }
            for tag in &record_tags {
                if tag == token {
                    score += 2.0;
                    matched = true;
                } else if tag.contains(token) {
                    score += 0.5;
                    matched = true;
                }
            }
        }
        for tag in &tags {
            if record_tags.iter().any(|record_tag| record_tag == tag) {
                score += 2.0;
                matched = true;
            }
        }
        if has_query && !matched {
            continue;
        }
        score += recency(record.source_seq);
        scored.push(ScoredMemory {
            record: record.clone(),
            score,
        });
    }

    scored.sort_by(|left, right| {
        right
            .score
            .total_cmp(&left.score)
            .then(left.record.id.cmp(&right.record.id))
    });
    scored.truncate(limit);
    scored
}

fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|character: char| {
            !character.is_alphanumeric() && character != '_' && character != '-'
        })
        .filter(|token| token.len() >= 2)
        .map(str::to_string)
        .collect()
}

fn recency(source_seq: Option<u64>) -> f32 {
    source_seq.unwrap_or(0).min(1000) as f32 * 0.0005
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(
        id: &str,
        kind: MemoryKind,
        text: &str,
        tags: &[&str],
        confidence: f32,
    ) -> MemoryRecord {
        MemoryRecord {
            id: id.to_string(),
            kind,
            text: text.to_string(),
            tags: tags.iter().map(|tag| tag.to_string()).collect(),
            source_seq: Some(10),
            confidence,
        }
    }

    fn map(records: Vec<MemoryRecord>) -> BTreeMap<String, MemoryRecord> {
        records
            .into_iter()
            .map(|record| (record.id.clone(), record))
            .collect()
    }

    #[test]
    fn tag_match_outranks_text_match_and_unrelated_is_excluded() {
        let memories = map(vec![
            record("tag", MemoryKind::Semantic, "something", &["rust"], 0.5),
            record("text", MemoryKind::Semantic, "uses rust", &[], 0.5),
            record("other", MemoryKind::Semantic, "python", &[], 0.9),
        ]);
        let results = recall(
            &memories,
            &MemoryQuery {
                text: "rust".to_string(),
                ..MemoryQuery::default()
            },
        );
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].record.id, "tag");
        assert_eq!(results[1].record.id, "text");
    }

    #[test]
    fn kinds_filter_and_limit_are_bounded() {
        let memories = map(vec![
            record("a", MemoryKind::Episodic, "rust one", &[], 0.5),
            record("b", MemoryKind::Episodic, "rust two", &[], 0.5),
            record("c", MemoryKind::Semantic, "rust three", &[], 0.5),
        ]);
        let results = recall(
            &memories,
            &MemoryQuery {
                text: "rust".to_string(),
                kinds: vec![MemoryKind::Episodic],
                limit: 1,
                ..MemoryQuery::default()
            },
        );
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].record.kind, MemoryKind::Episodic);
    }

    #[test]
    fn empty_query_returns_by_confidence() {
        let memories = map(vec![
            record("low", MemoryKind::Semantic, "a", &[], 0.2),
            record("high", MemoryKind::Semantic, "b", &[], 0.9),
        ]);
        let results = recall(&memories, &MemoryQuery::default());
        assert_eq!(results[0].record.id, "high");
    }
}
