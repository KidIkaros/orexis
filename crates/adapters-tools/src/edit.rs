//! Edit matching cascade: exact, whitespace/Unicode-normalized, bounded fuzzy.

use std::ops::Range;

use orexis_ports::{PortError, PortResult};
use unicode_normalization::UnicodeNormalization;

const FUZZY_THRESHOLD: f64 = 0.65;
const MAX_FUZZY_LINES: usize = 200;

#[derive(Debug, Clone, PartialEq)]
pub enum EditStrategy {
    Exact,
    Whitespace,
    Fuzzy(f64),
}

#[derive(Debug, Clone, PartialEq)]
pub struct EditOutcome {
    pub range: Range<usize>,
    pub strategy: EditStrategy,
}

impl EditOutcome {
    pub fn description(&self) -> String {
        match self.strategy {
            EditStrategy::Exact => "exact".to_string(),
            EditStrategy::Whitespace => "whitespace-normalized".to_string(),
            EditStrategy::Fuzzy(score) => format!("fuzzy {score:.2}"),
        }
    }
}

pub fn apply(text: &str, old: &str, new: &str) -> PortResult<(String, EditOutcome)> {
    if old.is_empty() {
        return Err(PortError("old text is empty".to_string()));
    }
    if let Some(outcome) = exact(text, old)? {
        return Ok((replace(text, &outcome.range, new), outcome));
    }
    if let Some(outcome) = normalized(text, old)? {
        return Ok((replace(text, &outcome.range, new), outcome));
    }
    if let Some(outcome) = fuzzy(text, old)? {
        return Ok((replace(text, &outcome.range, new), outcome));
    }
    Err(PortError("old text not found".to_string()))
}

fn replace(text: &str, range: &Range<usize>, new: &str) -> String {
    let mut out = String::with_capacity(text.len());
    out.push_str(&text[..range.start]);
    out.push_str(new);
    out.push_str(&text[range.end..]);
    out
}

fn exact(text: &str, old: &str) -> PortResult<Option<EditOutcome>> {
    let matches: Vec<usize> = text.match_indices(old).map(|(index, _)| index).collect();
    match matches.len() {
        0 => Ok(None),
        1 => Ok(Some(EditOutcome {
            range: matches[0]..matches[0] + old.len(),
            strategy: EditStrategy::Exact,
        })),
        count => Err(PortError(format!(
            "old text is ambiguous ({count} matches)"
        ))),
    }
}

fn normalized(text: &str, old: &str) -> PortResult<Option<EditOutcome>> {
    let spans = line_spans(text);
    let old_lines: Vec<String> = old.lines().map(normalize_line).collect();
    if old_lines.is_empty() {
        return Ok(None);
    }
    let text_lines: Vec<String> = spans
        .iter()
        .map(|(start, end)| normalize_line(&text[*start..*end]))
        .collect();
    if old_lines.len() > text_lines.len() {
        return Ok(None);
    }
    let mut found = Vec::new();
    for start in 0..=text_lines.len() - old_lines.len() {
        if text_lines[start..start + old_lines.len()] == old_lines[..] {
            found.push(start);
        }
    }
    match found.len() {
        0 => Ok(None),
        1 => {
            let start = found[0];
            Ok(Some(EditOutcome {
                range: line_range(text, &spans, start, old_lines.len(), old),
                strategy: EditStrategy::Whitespace,
            }))
        }
        count => Err(PortError(format!(
            "old text is ambiguous ({count} normalized matches)"
        ))),
    }
}

fn fuzzy(text: &str, old: &str) -> PortResult<Option<EditOutcome>> {
    let spans = line_spans(text);
    let old_lines: Vec<String> = old.lines().map(normalize_line).collect();
    if old_lines.is_empty() || old_lines.len() > MAX_FUZZY_LINES {
        return Ok(None);
    }
    let text_lines: Vec<String> = spans
        .iter()
        .map(|(start, end)| normalize_line(&text[*start..*end]))
        .collect();
    if old_lines.len() > text_lines.len() {
        return Ok(None);
    }
    let old_joined = old_lines.join("\n");
    let mut best: Option<(f64, usize)> = None;
    let mut ties = 0usize;
    for start in 0..=text_lines.len() - old_lines.len() {
        let candidate = text_lines[start..start + old_lines.len()].join("\n");
        let score = ratio(&candidate, &old_joined);
        if score < FUZZY_THRESHOLD {
            continue;
        }
        match best {
            None => {
                best = Some((score, start));
                ties = 1;
            }
            Some((best_score, _)) if score > best_score + 0.001 => {
                best = Some((score, start));
                ties = 1;
            }
            Some((best_score, _)) if (score - best_score).abs() <= 0.001 => {
                ties += 1;
            }
            _ => {}
        }
    }
    match best {
        None => Ok(None),
        Some(_) if ties > 1 => Err(PortError("old text is ambiguous (fuzzy)".to_string())),
        Some((score, start)) => Ok(Some(EditOutcome {
            range: line_range(text, &spans, start, old_lines.len(), old),
            strategy: EditStrategy::Fuzzy(score),
        })),
    }
}

fn normalize_line(line: &str) -> String {
    line.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .nfc()
        .collect()
}

fn line_spans(text: &str) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    let mut start = 0;
    for (index, _) in text.match_indices('\n') {
        spans.push((start, index + 1));
        start = index + 1;
    }
    if start < text.len() {
        spans.push((start, text.len()));
    }
    spans
}

fn line_range(
    text: &str,
    spans: &[(usize, usize)],
    start: usize,
    len: usize,
    old: &str,
) -> Range<usize> {
    let mut end = spans[start + len - 1].1;
    if !old.ends_with('\n') && end > 0 && text.as_bytes()[end - 1] == b'\n' {
        end -= 1;
    }
    spans[start].0..end
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut previous: Vec<usize> = (0..=b.len()).collect();
    let mut current = vec![0usize; b.len() + 1];
    for i in 1..=a.len() {
        current[0] = i;
        for j in 1..=b.len() {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            current[j] = (previous[j] + 1)
                .min(current[j - 1] + 1)
                .min(previous[j - 1] + cost);
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[b.len()]
}

fn ratio(a: &str, b: &str) -> f64 {
    let max = a.chars().count().max(b.chars().count());
    if max == 0 {
        return 1.0;
    }
    1.0 - (levenshtein(a, b) as f64 / max as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_wins() {
        let (updated, outcome) = apply("one two three", "two", "2").expect("edit");
        assert_eq!(updated, "one 2 three");
        assert_eq!(outcome.strategy, EditStrategy::Exact);
    }

    #[test]
    fn duplicate_exact_is_ambiguous() {
        assert!(apply("two two", "two", "2").is_err());
    }

    #[test]
    fn whitespace_drift_matches() {
        let text = "fn main() {\n    let x = 1;\n}\n";
        let old = "fn main() {\n        let x = 1;\n}";
        let (updated, outcome) = apply(text, old, "fn main() {\n    let x = 2;\n}").expect("edit");
        assert_eq!(outcome.strategy, EditStrategy::Whitespace);
        assert!(updated.contains("let x = 2"));
        assert!(!updated.contains("let x = 1"));
    }

    #[test]
    fn unicode_drift_matches() {
        let text = "caf\u{e9}\n";
        let old = "cafe\u{301}";
        let (updated, outcome) = apply(text, old, "tea").expect("edit");
        assert_eq!(outcome.strategy, EditStrategy::Whitespace);
        assert_eq!(updated, "tea\n");
    }

    #[test]
    fn fuzzy_typo_matches() {
        let text = "let value = compute();\n";
        let old = "let value = comput();";
        let (updated, outcome) = apply(text, old, "let value = compute(1);").expect("edit");
        assert!(matches!(outcome.strategy, EditStrategy::Fuzzy(_)));
        assert!(updated.contains("compute(1)"));
    }

    #[test]
    fn no_match_errors() {
        assert!(apply("hello\n", "absent", "x").is_err());
    }

    #[test]
    fn ambiguous_fuzzy_errors() {
        assert!(apply("alpha beta\nalpha beta\n", "alpha bet", "x").is_err());
    }
}
