//! Shadow-git checkpoints for turn-level undo.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use orexis_ports::{Checkpointer, PortError, PortResult};

pub struct GitCheckpointStore {
    root: PathBuf,
    git_dir: PathBuf,
}

impl GitCheckpointStore {
    pub fn new(root: impl Into<PathBuf>) -> PortResult<Self> {
        let root = fs::canonicalize(root.into())
            .map_err(|error| PortError(format!("checkpoint root: {error}")))?;
        let git_dir = root.join(".orexis").join("checkpoints.git");
        fs::create_dir_all(&git_dir)
            .map_err(|error| PortError(format!("checkpoint dir: {error}")))?;
        if !git_dir.join("HEAD").exists() {
            run_git(&root, &git_dir, &["init", "-q"])?;
            let info = git_dir.join("info");
            fs::create_dir_all(&info)
                .map_err(|error| PortError(format!("checkpoint info: {error}")))?;
            fs::write(info.join("exclude"), ".orexis/\ntarget/\nnode_modules/\n")
                .map_err(|error| PortError(format!("checkpoint exclude: {error}")))?;
        }
        Ok(Self { root, git_dir })
    }

    fn git(&self, args: &[&str]) -> PortResult<String> {
        run_git(&self.root, &self.git_dir, args)
    }
}

impl Checkpointer for GitCheckpointStore {
    fn create(&self, label: &str) -> PortResult<String> {
        self.git(&["add", "-A"])?;
        self.git(&["commit", "-q", "--allow-empty", "-m", label])?;
        Ok(self.git(&["rev-parse", "HEAD"])?.trim().to_string())
    }

    fn restore(&self, id: &str) -> PortResult<()> {
        self.git(&["reset", "--hard", id])?;
        let tracked = self.git(&["ls-tree", "-r", "--name-only", id])?;
        let keep: HashSet<PathBuf> = tracked.lines().map(PathBuf::from).collect();
        remove_untracked(&self.root, &self.root, &keep)?;
        Ok(())
    }

    fn diff(&self, id: &str) -> PortResult<String> {
        let diff = self.git(&["diff", "--no-color", id])?;
        Ok(diff.chars().take(64 * 1024).collect())
    }
}

fn run_git(root: &Path, git_dir: &Path, args: &[&str]) -> PortResult<String> {
    let output = Command::new("git")
        .arg("--git-dir")
        .arg(git_dir)
        .arg("--work-tree")
        .arg(root)
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_AUTHOR_NAME", "orexis")
        .env("GIT_AUTHOR_EMAIL", "orexis@localhost")
        .env("GIT_COMMITTER_NAME", "orexis")
        .env("GIT_COMMITTER_EMAIL", "orexis@localhost")
        .output()
        .map_err(|error| PortError(format!("git: {error}")))?;
    if !output.status.success() {
        return Err(PortError(format!(
            "git {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn remove_untracked(root: &Path, current: &Path, keep: &HashSet<PathBuf>) -> PortResult<()> {
    let entries = fs::read_dir(current)
        .map_err(|error| PortError(format!("read {}: {error}", current.display())))?;
    for entry in entries {
        let entry = entry.map_err(|error| PortError(error.to_string()))?;
        let path = entry.path();
        let name = entry.file_name();
        if current == root
            && matches!(
                name.to_str(),
                Some(".orexis" | ".git" | "target" | "node_modules")
            )
        {
            continue;
        }
        if path.is_dir() {
            remove_untracked(root, &path, keep)?;
        } else {
            let relative = path.strip_prefix(root).unwrap_or(&path).to_path_buf();
            if !keep.contains(&relative) {
                fs::remove_file(&path)
                    .map_err(|error| PortError(format!("remove {}: {error}", path.display())))?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_restore_round_trip() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("a.txt"), "one").expect("write");
        let store = GitCheckpointStore::new(dir.path()).expect("store");
        let id = store.create("turn-1").expect("checkpoint");

        std::fs::write(dir.path().join("a.txt"), "two").expect("write");
        std::fs::write(dir.path().join("b.txt"), "new").expect("write");
        store.restore(&id).expect("restore");

        assert_eq!(
            std::fs::read_to_string(dir.path().join("a.txt")).expect("read"),
            "one"
        );
        assert!(!dir.path().join("b.txt").exists());
    }

    #[test]
    fn diff_reports_the_change() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("a.txt"), "one").expect("write");
        let store = GitCheckpointStore::new(dir.path()).expect("store");
        let id = store.create("turn-1").expect("checkpoint");
        std::fs::write(dir.path().join("a.txt"), "two").expect("write");
        let diff = store.diff(&id).expect("diff");
        assert!(diff.contains("a.txt"));
    }
}
