//! Deterministic judges over the task's workspace (ported from the
//! harness eval — pass/fail comes from files, never from a model).

use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Judge {
    FileExists {
        path: String,
    },
    /// Negative check: the run must NOT have created this file.
    FileAbsent {
        path: String,
    },
    FileGrep {
        path: String,
        pattern: String,
    },
    FileEq {
        path: String,
        expected: String,
    },
}

impl Judge {
    /// Check one judge against the workspace root. A path that escapes
    /// the root is a judge bug, not a pass.
    pub fn check(&self, root: &Path) -> Result<bool, String> {
        match self {
            Judge::FileExists { path } => Ok(resolve(root, path)?.exists()),
            Judge::FileAbsent { path } => Ok(!resolve(root, path)?.exists()),
            Judge::FileGrep { path, pattern } => {
                let Ok(content) = std::fs::read_to_string(resolve(root, path)?) else {
                    return Ok(false);
                };
                let re = regex::Regex::new(pattern).map_err(|e| e.to_string())?;
                Ok(re.is_match(&content))
            }
            Judge::FileEq { path, expected } => {
                let Ok(content) = std::fs::read_to_string(resolve(root, path)?) else {
                    return Ok(false);
                };
                Ok(content.trim() == expected.trim())
            }
        }
    }
}

fn resolve(root: &Path, path: &str) -> Result<std::path::PathBuf, String> {
    let mut out = root.to_path_buf();
    for component in std::path::Path::new(path).components() {
        match component {
            std::path::Component::Normal(part) => out.push(part),
            std::path::Component::CurDir => {}
            _ => return Err(format!("judge path escapes the workspace: {path}")),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn judges_read_the_workspace() {
        let dir = tempfile::tempdir().expect("dir");
        std::fs::write(dir.path().join("a.txt"), "alpha beta").expect("write");
        let grep = Judge::FileGrep {
            path: "a.txt".into(),
            pattern: "beta".into(),
        };
        assert!(grep.check(dir.path()).expect("grep"));
        let absent = Judge::FileAbsent {
            path: "b.txt".into(),
        };
        assert!(absent.check(dir.path()).expect("absent"));
        let escapes = Judge::FileExists {
            path: "../x".into(),
        };
        assert!(escapes.check(dir.path()).is_err());
    }
}
