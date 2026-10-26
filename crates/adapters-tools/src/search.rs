//! Recursive text search.

use std::fs;
use std::path::{Path, PathBuf};

use orexis_core::Observation;
use orexis_ports::{PortError, PortResult};
use serde_json::Value;

use crate::{arg_str, LocalTools};

const MAX_SEARCH_RESULTS: usize = 100;
const MAX_SEARCH_FILE_BYTES: u64 = 1024 * 1024;
const MAX_SEARCH_DEPTH: usize = 20;

impl LocalTools {
    pub(crate) fn search(&self, args: &Value) -> PortResult<Observation> {
        let query = arg_str(args, "query")?;
        if query.is_empty() {
            return Err(PortError("query is empty".to_string()));
        }
        let start = self.resolve(args.get("path").and_then(Value::as_str).unwrap_or("."))?;
        let mut hits = Vec::new();
        self.walk(&start, query, 0, &mut hits)?;
        Ok(Observation {
            name: "search".to_string(),
            output: if hits.is_empty() {
                "no matches".to_string()
            } else {
                hits.join("\n")
            },
        })
    }

    fn walk(
        &self,
        path: &Path,
        query: &str,
        depth: usize,
        hits: &mut Vec<String>,
    ) -> PortResult<()> {
        if hits.len() >= MAX_SEARCH_RESULTS || depth > MAX_SEARCH_DEPTH {
            return Ok(());
        }
        let canonical = fs::canonicalize(path)
            .map_err(|error| PortError(format!("walk {}: {error}", path.display())))?;
        if !canonical.starts_with(&self.root) {
            return Ok(());
        }
        let metadata = fs::metadata(path)
            .map_err(|error| PortError(format!("stat {}: {error}", path.display())))?;
        if metadata.is_dir() {
            let mut entries: Vec<PathBuf> = fs::read_dir(path)
                .map_err(|error| PortError(format!("list {}: {error}", path.display())))?
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .collect();
            entries.sort();
            for entry in entries {
                let name = entry
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("");
                if matches!(name, ".git" | "target" | "node_modules") {
                    continue;
                }
                self.walk(&entry, query, depth + 1, hits)?;
                if hits.len() >= MAX_SEARCH_RESULTS {
                    break;
                }
            }
        } else if metadata.is_file() && metadata.len() <= MAX_SEARCH_FILE_BYTES {
            if let Ok(text) = fs::read_to_string(path) {
                for (index, line) in text.lines().enumerate() {
                    if line.contains(query) {
                        let relative = path.strip_prefix(&self.root).unwrap_or(path);
                        hits.push(format!(
                            "{}:{}: {}",
                            relative.display(),
                            index + 1,
                            line.trim()
                        ));
                        if hits.len() >= MAX_SEARCH_RESULTS {
                            return Ok(());
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
