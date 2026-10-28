//! The task corpus format: a fixture, a prompt, deterministic judges.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::judge::Judge;

/// One bench task: seeded fixture files, a natural-language prompt, and
/// deterministic judges run against the workspace afterwards.
/// `held_out` marks a variant kept for verification, not tuning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchTask {
    pub name: String,
    /// Independent subtasks the breadth shape decomposes into — the
    /// property the gate measures parallelism on.
    #[serde(default)]
    pub held_out: bool,
    #[serde(default)]
    pub fixture: Fixture,
    pub prompt: String,
    pub judges: Vec<Judge>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Fixture {
    #[serde(default)]
    pub files: BTreeMap<String, String>,
}

/// Load a suite file: a YAML list of tasks.
pub fn load(path: &std::path::Path) -> Result<Vec<BenchTask>, std::io::Error> {
    let text = std::fs::read_to_string(path)?;
    serde_yaml::from_str(&text).map_err(|error| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("{}: {error}", path.display()),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_task_yaml_parses() {
        let task: BenchTask = serde_yaml::from_str(
            r#"
name: breadth-01
held_out: false
fixture:
  files:
    src/a.txt: alpha
prompt: fix the files
judges:
  - type: file_grep
    path: src/a.txt
    pattern: beta
"#,
        )
        .expect("task");
        assert_eq!(task.name, "breadth-01");
        assert!(!task.held_out);
        assert_eq!(task.fixture.files["src/a.txt"], "alpha");
        assert_eq!(task.judges.len(), 1);
    }
}
