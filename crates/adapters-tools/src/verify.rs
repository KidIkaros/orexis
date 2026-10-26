//! Shell-backed verification for verify-on-stop.

use std::path::PathBuf;
use std::time::Duration;

use async_trait::async_trait;
use orexis_ports::{PortError, PortResult, Verifier, VerifyResult};

use crate::shell::run_shell;

pub struct ShellVerifier {
    root: PathBuf,
    command: String,
    timeout: Duration,
}

impl ShellVerifier {
    pub fn new(root: impl Into<PathBuf>, command: impl Into<String>) -> PortResult<Self> {
        let root = std::fs::canonicalize(root.into())
            .map_err(|error| PortError(format!("verify root: {error}")))?;
        Ok(Self {
            root,
            command: command.into(),
            timeout: Duration::from_secs(300),
        })
    }
}

#[async_trait]
impl Verifier for ShellVerifier {
    async fn verify(&self) -> PortResult<VerifyResult> {
        let root = self.root.clone();
        let command = self.command.clone();
        let timeout = self.timeout;
        let output = tokio::task::spawn_blocking(move || run_shell(&root, &command, timeout))
            .await
            .map_err(|error| PortError(error.to_string()))??;
        Ok(VerifyResult {
            command: self.command.clone(),
            passed: output.starts_with("exit 0\n"),
            output,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn verifier(command: &str) -> ShellVerifier {
        ShellVerifier::new(std::env::temp_dir(), command).expect("verifier")
    }

    #[tokio::test]
    async fn passing_command() {
        let result = verifier("true").verify().await.expect("verify");
        assert!(result.passed);
    }

    #[tokio::test]
    async fn failing_command() {
        let result = verifier("false").verify().await.expect("verify");
        assert!(!result.passed);
        assert!(result.output.contains("exit 1"));
    }

    #[tokio::test]
    async fn captures_output() {
        let result = verifier("echo hello").verify().await.expect("verify");
        assert!(result.passed);
        assert!(result.output.contains("hello"));
    }
}
