//! Compaction planning and the deterministic extractive summarizer.

use orexis_core::SessionEvent;
use orexis_ports::Summarizer;

/// Plan a compaction that summarizes everything except the last `keep_last`
/// events. The caller appends the returned event.
pub fn plan_compaction(
    events: &[SessionEvent],
    keep_last: usize,
    summarizer: &dyn Summarizer,
) -> Option<SessionEvent> {
    if events.len() <= keep_last {
        return None;
    }
    let split = events.len() - keep_last;
    let range_start_seq = events
        .iter()
        .rev()
        .find_map(|event| match event {
            SessionEvent::Compaction { range_end_seq, .. } => Some(*range_end_seq + 1),
            _ => None,
        })
        .unwrap_or(1);
    let summary = summarizer.summarize(&events[..split]);
    Some(SessionEvent::Compaction {
        range_start_seq,
        range_end_seq: split as u64,
        summary,
    })
}

/// Deterministic extractive summarizer for M1; an LLM summarizer replaces it
/// behind the same port later.
pub struct ExtractiveSummarizer;

impl Summarizer for ExtractiveSummarizer {
    fn summarize(&self, events: &[SessionEvent]) -> String {
        let mut parts = Vec::new();
        for event in events {
            match event {
                SessionEvent::UserMessage { text } => {
                    parts.push(format!("user: {}", first_line(text)));
                }
                SessionEvent::ModelResponse { text, .. } if !text.is_empty() => {
                    parts.push(format!("agent: {}", first_line(text)));
                }
                SessionEvent::Done { summary } => {
                    parts.push(format!("done: {}", first_line(summary)));
                }
                SessionEvent::Compaction { summary, .. } => {
                    parts.push(format!("summary: {}", first_line(summary)));
                }
                _ => {}
            }
        }
        parts.join(" | ").chars().take(2000).collect()
    }
}

fn first_line(text: &str) -> &str {
    text.lines().next().unwrap_or("").trim()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compaction_keeps_the_tail() {
        let events = vec![
            SessionEvent::UserMessage {
                text: "one".to_string(),
            },
            SessionEvent::ModelResponse {
                text: "two".to_string(),
                actions: Vec::new(),
            },
            SessionEvent::UserMessage {
                text: "three".to_string(),
            },
        ];
        let compaction = plan_compaction(&events, 1, &ExtractiveSummarizer).unwrap();
        let SessionEvent::Compaction {
            range_end_seq,
            summary,
            ..
        } = compaction
        else {
            panic!("wrong variant");
        };
        assert_eq!(range_end_seq, 2);
        assert!(summary.contains("user: one"));
        assert!(summary.contains("agent: two"));
        assert!(!summary.contains("three"));
    }

    #[test]
    fn compaction_is_skipped_when_short() {
        let events = vec![SessionEvent::UserMessage {
            text: "one".to_string(),
        }];
        assert!(plan_compaction(&events, 8, &ExtractiveSummarizer).is_none());
    }

    #[test]
    fn second_compaction_starts_after_the_first() {
        let events = vec![
            SessionEvent::UserMessage {
                text: "one".to_string(),
            },
            SessionEvent::ModelResponse {
                text: "two".to_string(),
                actions: Vec::new(),
            },
            SessionEvent::UserMessage {
                text: "three".to_string(),
            },
        ];
        let first = plan_compaction(&events, 1, &ExtractiveSummarizer).unwrap();
        let mut events2 = events.clone();
        events2.push(first);
        events2.push(SessionEvent::UserMessage {
            text: "four".to_string(),
        });

        let second = plan_compaction(&events2, 1, &ExtractiveSummarizer).unwrap();
        let SessionEvent::Compaction {
            range_start_seq,
            range_end_seq,
            ..
        } = second
        else {
            panic!("wrong variant");
        };
        assert_eq!(range_start_seq, 3);
        assert_eq!(range_end_seq, 4);
    }
}
