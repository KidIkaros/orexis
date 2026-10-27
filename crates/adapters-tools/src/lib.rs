//! Local coding tools with a path sandbox.

pub mod checkpoints;
mod edit;
mod files;
pub mod gate;
mod git;
pub mod router;
mod search;
mod shell;
mod strategy_tools;
pub mod verify;
mod workspace;

pub use checkpoints::GitCheckpointStore;
pub use gate::{AutoApprove, DenyAll, GatedTools};
pub use router::{NamespaceTools, RouterTools};
pub use strategy_tools::StrategyTools;
pub use verify::ShellVerifier;

use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use orexis_core::{Observation, ToolSpec};
use orexis_ports::{Lsp, PortError, PortResult, ToolExecutor};
use serde_json::{json, Value};

pub struct LocalTools {
    root: PathBuf,
    lsp: Option<Arc<dyn Lsp>>,
}

impl LocalTools {
    pub fn new(root: impl Into<PathBuf>) -> PortResult<Self> {
        let root = root.into();
        let root = fs::canonicalize(&root)
            .map_err(|error| PortError(format!("workspace root: {error}")))?;
        Ok(Self { root, lsp: None })
    }

    pub fn with_lsp(root: impl Into<PathBuf>, lsp: Arc<dyn Lsp>) -> PortResult<Self> {
        let mut tools = Self::new(root)?;
        tools.lsp = Some(lsp);
        Ok(tools)
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
                name: "git_status".to_string(),
                description: "Read-only git status for the workspace".to_string(),
                schema: json!({ "type": "object", "properties": {} }),
            },
            ToolSpec {
                name: "git_diff".to_string(),
                description: "Read-only git diff for the workspace".to_string(),
                schema: json!({
                    "type": "object",
                    "properties": { "path": { "type": "string" } }
                }),
            },
            ToolSpec {
                name: "shell".to_string(),
                description: "Run a shell command at the workspace root".to_string(),
                schema: json!({
                    "type": "object",
                    "properties": {
                        "command": { "type": "string" },
                        "timeout_secs": { "type": "integer" },
                        "cwd": { "type": "string" }
                    },
                    "required": ["command"]
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

    pub(crate) fn resolve(&self, path: &str) -> PortResult<PathBuf> {
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
}

#[async_trait]
impl ToolExecutor for LocalTools {
    async fn execute(&self, name: &str, args: &str) -> PortResult<Observation> {
        let value: Value = serde_json::from_str(args)
            .map_err(|error| PortError(format!("invalid args: {error}")))?;
        let observation = match name {
            "read_file" => self.read_file(&value),
            "write_file" => self.write_file(&value),
            "edit_file" => self.edit_file(&value).await,
            "list_dir" => self.list_dir(&value),
            "search" => self.search(&value),
            "shell" => self.shell(&value).await,
            "git_status" => self.git_status(&value),
            "git_diff" => self.git_diff(&value),
            other => Err(PortError(format!("unknown tool '{other}'"))),
        }?;
        Ok(Observation {
            name: observation.name,
            output: delimit(name, &observation.output),
        })
    }
}

/// Built-in tool output is untrusted content (invariant 22): repository
/// files, command output, and listings enter the model context delimited,
/// exactly as external tool output does. A closing tag inside the output
/// must not end the delimited region early, so it is escaped.
fn delimit(name: &str, output: &str) -> String {
    let escaped = output.replace("</tool_result", "<\\/tool_result");
    format!("<tool_result name=\"{name}\">\n{escaped}\n</tool_result>")
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
