//! Read-only workspace browsing over the same path sandbox.

use std::fs;
use std::process::Command;

use orexis_ports::{FileEntry, FilePreview, PortError, PortResult, Workspace};

use crate::LocalTools;

const MAX_READ_BYTES: usize = 64 * 1024;
const MAX_ENTRIES: usize = 500;

impl Workspace for LocalTools {
    fn list(&self, path: &str) -> PortResult<Vec<FileEntry>> {
        let dir = self.resolve(path)?;
        let mut entries = Vec::new();
        for entry in fs::read_dir(&dir)
            .map_err(|error| PortError(format!("list {}: {error}", dir.display())))?
        {
            let entry = entry.map_err(|error| PortError(error.to_string()))?;
            let name = entry.file_name().to_string_lossy().to_string();
            if matches!(
                name.as_str(),
                ".git" | ".orexis" | "target" | "node_modules"
            ) {
                continue;
            }
            let metadata = entry
                .metadata()
                .map_err(|error| PortError(format!("stat {}: {error}", name)))?;
            let relative = entry
                .path()
                .strip_prefix(&self.root)
                .unwrap_or(&entry.path())
                .display()
                .to_string();
            entries.push(FileEntry {
                name,
                path: relative,
                is_dir: metadata.is_dir(),
                size: metadata.len(),
            });
        }
        entries.sort_by(|left, right| {
            right
                .is_dir
                .cmp(&left.is_dir)
                .then(left.name.cmp(&right.name))
        });
        entries.truncate(MAX_ENTRIES);
        Ok(entries)
    }

    fn read(&self, path: &str, max_bytes: usize) -> PortResult<FilePreview> {
        let file = self.resolve(path)?;
        let bytes = fs::read(&file)
            .map_err(|error| PortError(format!("read {}: {error}", file.display())))?;
        let limit = max_bytes.min(MAX_READ_BYTES);
        let truncated = bytes.len() > limit;
        let content = String::from_utf8_lossy(&bytes[..bytes.len().min(limit)]).to_string();
        let relative = file
            .strip_prefix(&self.root)
            .unwrap_or(&file)
            .display()
            .to_string();
        Ok(FilePreview {
            path: relative,
            content,
            truncated,
        })
    }

    fn changed(&self) -> PortResult<Vec<String>> {
        let output = Command::new("git")
            .arg("-C")
            .arg(&self.root)
            .args(["status", "--porcelain"])
            .output();
        let Ok(output) = output else {
            return Ok(Vec::new());
        };
        if !output.status.success() {
            return Ok(Vec::new());
        }
        let text = String::from_utf8_lossy(&output.stdout);
        let mut paths = Vec::new();
        for line in text.lines() {
            let rest = line.get(3..).unwrap_or("").trim();
            let path = rest.rsplit(" -> ").next().unwrap_or(rest);
            if !path.is_empty() {
                paths.push(path.to_string());
            }
        }
        Ok(paths)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_and_reads_inside_the_root() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("a.txt"), "hello").expect("write");
        std::fs::create_dir(dir.path().join("src")).expect("mkdir");
        let tools = LocalTools::new(dir.path()).expect("tools");
        let entries = tools.list(".").expect("list");
        assert!(entries.iter().any(|entry| entry.name == "a.txt"));
        assert!(entries[0].is_dir, "directories sort first");
        let preview = tools.read("a.txt", 1024).expect("read");
        assert_eq!(preview.content, "hello");
        assert!(!preview.truncated);
    }

    #[test]
    fn rejects_path_escapes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let tools = LocalTools::new(dir.path()).expect("tools");
        assert!(tools.list("../").is_err());
        assert!(tools.read("../secret", 1024).is_err());
    }

    #[test]
    fn caps_large_files() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("big.txt"), "x".repeat(4096)).expect("write");
        let tools = LocalTools::new(dir.path()).expect("tools");
        let preview = tools.read("big.txt", 10).expect("read");
        assert!(preview.truncated);
        assert_eq!(preview.content.len(), 10);
    }

    #[test]
    fn changed_reports_git_status() {
        let dir = tempfile::tempdir().expect("tempdir");
        let run = |args: &[&str]| {
            Command::new("git")
                .arg("-C")
                .arg(dir.path())
                .args(args)
                .output()
                .expect("git")
        };
        assert!(run(&["init", "-q"]).status.success());
        std::fs::write(dir.path().join("a.txt"), "one").expect("write");
        assert!(run(&["add", "-A"]).status.success());
        assert!(run(&[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@localhost",
            "commit",
            "-q",
            "-m",
            "init",
        ])
        .status
        .success());
        std::fs::write(dir.path().join("a.txt"), "two").expect("write");
        let tools = LocalTools::new(dir.path()).expect("tools");
        let changed = tools.changed().expect("changed");
        assert!(changed.iter().any(|path| path == "a.txt"));
    }
}
