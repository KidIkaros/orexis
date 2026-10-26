//! Filesystem context discovery: AGENTS.md, CLAUDE.md, .orexis/CONTEXT.md,
//! from the project root down to the working directory.

use std::path::{Path, PathBuf};

use orexis_core::ContextFile;
use orexis_ports::ContextSource;

pub struct FileContextSource {
    root: PathBuf,
    cwd: PathBuf,
}

impl FileContextSource {
    pub fn new(root: impl Into<PathBuf>, cwd: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            cwd: cwd.into(),
        }
    }

    fn candidates(dir: &Path) -> [PathBuf; 3] {
        [
            dir.join("AGENTS.md"),
            dir.join("CLAUDE.md"),
            dir.join(".orexis").join("CONTEXT.md"),
        ]
    }

    fn ancestry(&self) -> Vec<PathBuf> {
        let mut dirs = Vec::new();
        let mut current = self.cwd.clone();
        loop {
            dirs.push(current.clone());
            if current == self.root {
                break;
            }
            match current.parent() {
                Some(parent) if parent.starts_with(&self.root) => {
                    current = parent.to_path_buf();
                }
                _ => break,
            }
        }
        dirs.reverse();
        dirs
    }
}

impl ContextSource for FileContextSource {
    fn load(&self) -> Vec<ContextFile> {
        let mut files = Vec::new();
        for dir in self.ancestry() {
            for candidate in Self::candidates(&dir) {
                if candidate.is_file() {
                    if let Ok(content) = std::fs::read_to_string(&candidate) {
                        files.push(ContextFile {
                            path: candidate.display().to_string(),
                            content,
                        });
                    }
                }
            }
        }
        files
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_root_to_cwd_in_order() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let nested = root.join("crates").join("app");
        std::fs::create_dir_all(nested.join(".orexis")).unwrap();
        std::fs::write(root.join("AGENTS.md"), "root rules").unwrap();
        std::fs::write(nested.join("AGENTS.md"), "app rules").unwrap();
        std::fs::write(nested.join(".orexis").join("CONTEXT.md"), "app context").unwrap();

        let source = FileContextSource::new(root, &nested);
        let files = source.load();
        assert_eq!(files.len(), 3);
        assert!(files[0].content.contains("root rules"));
        assert!(files[1].content.contains("app rules"));
        assert!(files[2].content.contains("app context"));
    }

    #[test]
    fn empty_when_no_context_files_exist() {
        let dir = tempfile::tempdir().unwrap();
        let source = FileContextSource::new(dir.path(), dir.path());
        assert!(source.load().is_empty());
    }
}
