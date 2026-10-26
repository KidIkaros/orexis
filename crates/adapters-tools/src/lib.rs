//! Local coding tools with a path sandbox.

use std::fs;
use std::path::{Component, Path, PathBuf};

use async_trait::async_trait;
use orexis_core::{Observation, ToolSpec};
use orexis_ports::{PortError, PortResult, ToolExecutor};
use serde_json::{json, Value};

const MAX_READ_BYTES: usize = 64 * 1024;
const MAX_LIST_ENTRIES: usize = 200;
const MAX_SEARCH_RESULTS: usize = 100;
const MAX_SEARCH_FILE_BYTES: u64 = 1024 * 1024;
const MAX_SEARCH_DEPTH: usize = 20;

pub struct LocalTools {
    root: PathBuf,
}

impl LocalTools {
    pub fn new(root: impl Into<PathBuf>) -> PortResult<Self> {
        let root = root.into();
        let root = fs::canonicalize(&root)
            .map_err(|error| PortError(format!("workspace root: {error}")))?;
        Ok(Self { root })
    }

    pub fn specs() -> Vec<ToolSpec> {
        vec![
            ToolSpec {
                name: "read_file".to_string(),
                description: "Read a UTF-8 file inside the workspace".to_string(),
                schema: json!({
                    "type": "object",
                    "properties": { "path": { "type": "string" } },
                    "required": ["path"]
                }),
            },
            ToolSpec {
                name: "write_file".to_string(),
                description: "Write a UTF-8 file inside the workspace".to_string(),
                schema: json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string" },
                        "content": { "type": "string" }
                    },
                    "required": ["path", "content"]
                }),
            },
            ToolSpec {
                name: "edit_file".to_string(),
                description: "Replace one unique substring in a file".to_string(),
                schema: json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string" },
                        "old": { "type": "string" },
                        "new": { "type": "string" }
                    },
                    "required": ["path", "old", "new"]
                }),
            },
            ToolSpec {
                name: "list_dir".to_string(),
                description: "List a directory inside the workspace".to_string(),
                schema: json!({
                    "type": "object",
                    "properties": { "path": { "type": "string" } }
                }),
            },
            ToolSpec {
                name: "search".to_string(),
                description: "Search text files inside the workspace".to_string(),
                schema: json!({
                    "type": "object",
                    "properties": {
                        "query": { "type": "string" },
                        "path": { "type": "string" }
                    },
                    "required": ["query"]
                }),
            },
        ]
    }

    fn resolve(&self, path: &str) -> PortResult<PathBuf> {
        let candidate = if Path::new(path).is_absolute() {
            PathBuf::from(path)
        } else {
            self.root.join(path)
        };
        let normalized = normalize(&candidate);
        if !normalized.starts_with(&self.root) {
            return Err(PortError(format!("path escapes the workspace: {path}")));
        }
        if normalized.exists() {
            let canonical = fs::canonicalize(&normalized)
                .map_err(|error| PortError(format!("resolve {path}: {error}")))?;
            if !canonical.starts_with(&self.root) {
                return Err(PortError(format!("path escapes the workspace: {path}")));
            }
            return Ok(canonical);
        }
        let parent = normalized
            .parent()
            .ok_or_else(|| PortError(format!("invalid path: {path}")))?;
        let parent = fs::canonicalize(parent)
            .map_err(|error| PortError(format!("resolve parent of {path}: {error}")))?;
        if !parent.starts_with(&self.root) {
            return Err(PortError(format!("path escapes the workspace: {path}")));
        }
        let name = normalized
            .file_name()
            .ok_or_else(|| PortError(format!("invalid path: {path}")))?;
        Ok(parent.join(name))
    }

    fn read_file(&self, args: &Value) -> PortResult<Observation> {
        let path = self.resolve(arg_str(args, "path")?)?;
        let bytes = fs::read(&path)
            .map_err(|error| PortError(format!("read {}: {error}", path.display())))?;
        let limit = args
            .get("max_bytes")
            .and_then(Value::as_u64)
            .map(|value| value as usize)
            .unwrap_or(MAX_READ_BYTES)
            .min(MAX_READ_BYTES);
        let truncated = bytes.len() > limit;
        let text = String::from_utf8_lossy(&bytes[..bytes.len().min(limit)]).to_string();
        let output = if truncated {
            format!("{text}\n[truncated at {limit} bytes]")
        } else {
            text
        };
        Ok(Observation {
            name: "read_file".to_string(),
            output,
        })
    }

    fn write_file(&self, args: &Value) -> PortResult<Observation> {
        let path = self.resolve(arg_str(args, "path")?)?;
        let content = arg_str(args, "content")?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| PortError(format!("mkdir {}: {error}", parent.display())))?;
        }
        fs::write(&path, content)
            .map_err(|error| PortError(format!("write {}: {error}", path.display())))?;
        Ok(Observation {
            name: "write_file".to_string(),
            output: format!(
                "wrote {} bytes to {}",
                content.len(),
                path.strip_prefix(&self.root).unwrap_or(&path).display()
            ),
        })
    }

    fn edit_file(&self, args: &Value) -> PortResult<Observation> {
        let path = self.resolve(arg_str(args, "path")?)?;
        let old = arg_str(args, "old")?;
        let new = arg_str(args, "new")?;
        if old.is_empty() {
            return Err(PortError("old text is empty".to_string()));
        }
        let text = fs::read_to_string(&path)
            .map_err(|error| PortError(format!("read {}: {error}", path.display())))?;
        let count = text.matches(old).count();
        if count == 0 {
            return Err(PortError(format!(
                "old text not found in {}",
                path.display()
            )));
        }
        if count > 1 {
            return Err(PortError(format!(
                "old text is ambiguous in {} ({count} matches)",
                path.display()
            )));
        }
        let updated = text.replacen(old, new, 1);
        fs::write(&path, updated)
            .map_err(|error| PortError(format!("write {}: {error}", path.display())))?;
        Ok(Observation {
            name: "edit_file".to_string(),
            output: format!(
                "edited {}",
                path.strip_prefix(&self.root).unwrap_or(&path).display()
            ),
        })
    }

    fn list_dir(&self, args: &Value) -> PortResult<Observation> {
        let path = self.resolve(args.get("path").and_then(Value::as_str).unwrap_or("."))?;
        let mut entries = Vec::new();
        let entries_iter = fs::read_dir(&path)
            .map_err(|error| PortError(format!("list {}: {error}", path.display())))?;
        for entry in entries_iter {
            let entry = entry.map_err(|error| PortError(error.to_string()))?;
            let name = entry.file_name().to_string_lossy().to_string();
            let file_type = entry
                .file_type()
                .map_err(|error| PortError(error.to_string()))?;
            if file_type.is_dir() {
                entries.push(format!("{name}/"));
            } else {
                let size = entry.metadata().map(|metadata| metadata.len()).unwrap_or(0);
                entries.push(format!("{name}  {size}"));
            }
            if entries.len() >= MAX_LIST_ENTRIES {
                break;
            }
        }
        entries.sort();
        Ok(Observation {
            name: "list_dir".to_string(),
            output: if entries.is_empty() {
                "(empty)".to_string()
            } else {
                entries.join("\n")
            },
        })
    }

    fn search(&self, args: &Value) -> PortResult<Observation> {
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

#[async_trait]
impl ToolExecutor for LocalTools {
    async fn execute(&self, name: &str, args: &str) -> PortResult<Observation> {
        let value: Value = serde_json::from_str(args)
            .map_err(|error| PortError(format!("invalid args: {error}")))?;
        match name {
            "read_file" => self.read_file(&value),
            "write_file" => self.write_file(&value),
            "edit_file" => self.edit_file(&value),
            "list_dir" => self.list_dir(&value),
            "search" => self.search(&value),
            other => Err(PortError(format!("unknown tool '{other}'"))),
        }
    }
}

fn arg_str<'a>(value: &'a Value, key: &str) -> PortResult<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| PortError(format!("missing '{key}'")))
}

fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            Component::Prefix(prefix) => out.push(prefix.as_os_str()),
            Component::RootDir => out.push(Component::RootDir.as_os_str()),
            Component::Normal(part) => out.push(part),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::executor::block_on;

    fn setup() -> (tempfile::TempDir, LocalTools) {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("a.txt"), "hello\nworld\n").expect("write");
        std::fs::create_dir(dir.path().join("dir")).expect("mkdir");
        std::fs::write(dir.path().join("dir/b.txt"), "needle\n").expect("write");
        let tools = LocalTools::new(dir.path()).expect("tools");
        (dir, tools)
    }

    fn run(tools: &LocalTools, name: &str, args: &str) -> PortResult<Observation> {
        block_on(tools.execute(name, args))
    }

    #[test]
    fn read_write_edit_round_trip() {
        let (_dir, tools) = setup();
        let read = run(&tools, "read_file", r#"{"path":"a.txt"}"#).expect("read");
        assert!(read.output.contains("hello"));

        run(&tools, "write_file", r#"{"path":"c.txt","content":"new"}"#).expect("write");
        let written = run(&tools, "read_file", r#"{"path":"c.txt"}"#).expect("read");
        assert_eq!(written.output, "new");

        run(
            &tools,
            "edit_file",
            r#"{"path":"a.txt","old":"world","new":"there"}"#,
        )
        .expect("edit");
        let edited = run(&tools, "read_file", r#"{"path":"a.txt"}"#).expect("read");
        assert!(edited.output.contains("there"));
        assert!(!edited.output.contains("world"));
    }

    #[test]
    fn edit_rejects_absent_and_ambiguous() {
        let (_dir, tools) = setup();
        assert!(run(
            &tools,
            "edit_file",
            r#"{"path":"a.txt","old":"nope","new":"x"}"#
        )
        .is_err());
        assert!(run(
            &tools,
            "edit_file",
            r#"{"path":"a.txt","old":"l","new":"x"}"#
        )
        .is_err());
    }

    #[test]
    fn list_and_search_work() {
        let (_dir, tools) = setup();
        let listed = run(&tools, "list_dir", r#"{"path":"."}"#).expect("list");
        assert!(listed.output.contains("a.txt"));
        assert!(listed.output.contains("dir/"));

        let found = run(&tools, "search", r#"{"query":"needle"}"#).expect("search");
        assert!(found.output.contains("dir/b.txt:1"));
    }

    #[test]
    fn rejects_relative_escape() {
        let (_dir, tools) = setup();
        assert!(run(&tools, "read_file", r#"{"path":"../outside.txt"}"#).is_err());
    }

    #[test]
    fn rejects_absolute_outside() {
        let (_dir, tools) = setup();
        assert!(run(&tools, "read_file", r#"{"path":"/etc/passwd"}"#).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlink_escape() {
        let outside = tempfile::tempdir().expect("outside");
        std::fs::write(outside.path().join("secret"), "secret").expect("write");
        let (dir, tools) = setup();
        std::os::unix::fs::symlink(outside.path(), dir.path().join("link")).expect("symlink");
        assert!(run(&tools, "read_file", r#"{"path":"link/secret"}"#).is_err());
    }

    #[test]
    fn read_truncates_at_the_cap() {
        let (dir, tools) = setup();
        let big = "x".repeat(70 * 1024);
        std::fs::write(dir.path().join("big.txt"), big).expect("write");
        let read = run(&tools, "read_file", r#"{"path":"big.txt"}"#).expect("read");
        assert!(read.output.contains("[truncated at 65536 bytes]"));
    }

    #[test]
    fn unknown_tool_is_an_error() {
        let (_dir, tools) = setup();
        assert!(run(&tools, "nope", "{}").is_err());
    }
}
