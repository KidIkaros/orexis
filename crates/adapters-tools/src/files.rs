//! File tools: read, write, edit, list.

use std::fs;

use orexis_core::Observation;
use orexis_ports::{Diagnostic, PortError, PortResult};
use serde_json::Value;

use crate::{arg_str, LocalTools};

const MAX_READ_BYTES: usize = 64 * 1024;
const MAX_LIST_ENTRIES: usize = 200;

impl LocalTools {
    pub(crate) fn read_file(&self, args: &Value) -> PortResult<Observation> {
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

    pub(crate) fn write_file(&self, args: &Value) -> PortResult<Observation> {
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

    pub(crate) async fn edit_file(&self, args: &Value) -> PortResult<Observation> {
        let path = self.resolve(arg_str(args, "path")?)?;
        let old = arg_str(args, "old")?;
        let new = arg_str(args, "new")?;
        let text = fs::read_to_string(&path)
            .map_err(|error| PortError(format!("read {}: {error}", path.display())))?;
        let (updated, outcome) = crate::edit::apply(&text, old, new)?;
        fs::write(&path, &updated)
            .map_err(|error| PortError(format!("write {}: {error}", path.display())))?;
        let mut output = format!(
            "edited {} ({})",
            path.strip_prefix(&self.root).unwrap_or(&path).display(),
            outcome.description()
        );
        if let Some(lsp) = &self.lsp {
            let relative = path
                .strip_prefix(&self.root)
                .unwrap_or(&path)
                .display()
                .to_string();
            if let Ok(diagnostics) = lsp.diagnostics(&relative, &updated).await {
                append_diagnostics(&mut output, &diagnostics);
            }
        }
        Ok(Observation {
            name: "edit_file".to_string(),
            output,
        })
    }

    pub(crate) fn list_dir(&self, args: &Value) -> PortResult<Observation> {
        let path = self.resolve(args.get("path").and_then(Value::as_str).unwrap_or("."))?;
        let mut entries = Vec::new();
        let entries_iter = fs::read_dir(&path)
            .map_err(|error| PortError(format!("list {}: {error}", path.display())))?;
        for entry in entries_iter {
            let entry = entry.map_err(|error| PortError(error.to_string()))?;
            let name = entry.file_name().to_string_lossy().to_string();
            // Harness state is not the project: advertising it cost real
            // steps (M8-16). target/ and node_modules/ stay listable.
            if matches!(name.as_str(), ".orexis" | ".git") {
                continue;
            }
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
}

fn append_diagnostics(output: &mut String, diagnostics: &[Diagnostic]) {
    if diagnostics.is_empty() {
        return;
    }
    output.push_str("\n--- diagnostics ---");
    for diagnostic in diagnostics {
        output.push_str(&format!(
            "\n{}:{}:{} {}: {}",
            diagnostic.path,
            diagnostic.line + 1,
            diagnostic.column + 1,
            diagnostic.severity,
            diagnostic.message
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn write_creates_missing_parents() {
        let dir = tempfile::tempdir().expect("tempdir");
        let tools = LocalTools::new(dir.path()).expect("tools");
        tools
            .write_file(&json!({
                "path": "reports/north.txt",
                "content": "margin: 4",
            }))
            .expect("write through missing dirs");
        assert_eq!(
            std::fs::read_to_string(dir.path().join("reports/north.txt")).expect("read"),
            "margin: 4"
        );
    }

    #[test]
    fn write_still_rejects_escapes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let tools = LocalTools::new(dir.path()).expect("tools");
        assert!(tools
            .write_file(&json!({
                "path": "../outside/x.txt",
                "content": "nope",
            }))
            .is_err());
    }
}
