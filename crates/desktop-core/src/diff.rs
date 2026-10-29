//! Pure unified and hunk-aligned split diff model for the desktop workspace.

use serde::{Deserialize, Serialize};

/// Auto-expand threshold (in lines) for opening a diff full-screen.
pub const AUTO_EXPAND_LINE_THRESHOLD: usize = 40;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiffLineKind {
    Meta,
    Hunk,
    Ctx,
    Del,
    Add,
    Pad,
}

impl DiffLineKind {
    pub fn css_class(self) -> &'static str {
        match self {
            Self::Meta => "meta",
            Self::Hunk => "hunk",
            Self::Ctx => "ctx",
            Self::Del => "del",
            Self::Add => "add",
            Self::Pad => "pad",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffCell {
    pub kind: DiffLineKind,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SplitRow {
    pub left: DiffCell,
    pub right: DiffCell,
}

pub fn classify_diff_line(line: &str) -> DiffLineKind {
    if line.starts_with("+++")
        || line.starts_with("---")
        || line.starts_with("diff --git")
        || line.starts_with("index ")
    {
        DiffLineKind::Meta
    } else if line.starts_with("@@") {
        DiffLineKind::Hunk
    } else if line.starts_with('+') {
        DiffLineKind::Add
    } else if line.starts_with('-') {
        DiffLineKind::Del
    } else {
        DiffLineKind::Ctx
    }
}

pub fn should_auto_expand_diff(diff: &str) -> bool {
    diff.lines().count() > AUTO_EXPAND_LINE_THRESHOLD
}

/// Aligns a unified diff into side-by-side rows so deletions and additions
/// within each modification block line up row-for-row, with blank padding
/// where a block is unbalanced.
pub fn align_split_diff(diff: &str) -> Vec<SplitRow> {
    let lines: Vec<&str> = diff.lines().collect();
    let mut rows = Vec::new();
    let mut index = 0;

    while index < lines.len() {
        let line = lines[index];

        // Pair `--- a/...` and `+++ b/...` headers onto one aligned Meta row.
        if line.starts_with("---") && index + 1 < lines.len() && lines[index + 1].starts_with("+++")
        {
            rows.push(SplitRow {
                left: DiffCell {
                    kind: DiffLineKind::Meta,
                    text: line.to_string(),
                },
                right: DiffCell {
                    kind: DiffLineKind::Meta,
                    text: lines[index + 1].to_string(),
                },
            });
            index += 2;
            continue;
        }

        let kind = classify_diff_line(line);
        match kind {
            DiffLineKind::Meta | DiffLineKind::Hunk | DiffLineKind::Ctx => {
                rows.push(SplitRow {
                    left: DiffCell {
                        kind,
                        text: line.to_string(),
                    },
                    right: DiffCell {
                        kind,
                        text: line.to_string(),
                    },
                });
                index += 1;
            }
            DiffLineKind::Del | DiffLineKind::Add => {
                let mut dels = Vec::new();
                let mut adds = Vec::new();
                while index < lines.len() {
                    let current = lines[index];
                    match classify_diff_line(current) {
                        DiffLineKind::Del => {
                            dels.push(current.to_string());
                            index += 1;
                        }
                        DiffLineKind::Add => {
                            adds.push(current.to_string());
                            index += 1;
                        }
                        _ => break,
                    }
                }
                let count = dels.len().max(adds.len());
                for i in 0..count {
                    let left = match dels.get(i) {
                        Some(text) => DiffCell {
                            kind: DiffLineKind::Del,
                            text: text.clone(),
                        },
                        None => DiffCell {
                            kind: DiffLineKind::Pad,
                            text: String::new(),
                        },
                    };
                    let right = match adds.get(i) {
                        Some(text) => DiffCell {
                            kind: DiffLineKind::Add,
                            text: text.clone(),
                        },
                        None => DiffCell {
                            kind: DiffLineKind::Pad,
                            text: String::new(),
                        },
                    };
                    rows.push(SplitRow { left, right });
                }
            }
            DiffLineKind::Pad => {
                index += 1;
            }
        }
    }

    rows
}
