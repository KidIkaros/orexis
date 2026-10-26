//! Read-only git tools.

use std::process::Command;

use orexis_core::Observation;
use orexis_ports::{PortError, PortResult};
use serde_json::Value;

use crate::shell::SHELL_OUTPUT_CAP;
use crate::LocalTools;

impl LocalTools {
    pub(crate) fn git_status(&self, _args: &Value) -> PortResult<Observation> {
        let output = Command::new("git")
            .arg("-C")
            .arg(&self.root)
            .arg("status")
            .arg("--porcelain")
            .output()
            .map_err(|error| PortError(format!("git status: {error}")))?;
        if !output.status.success() {
            return Err(PortError(format!(
                "git status: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        Ok(Observation {
            name: "git_status".to_string(),
            output: String::from_utf8_lossy(&output.stdout).to_string(),
        })
    }

    pub(crate) fn git_diff(&self, args: &Value) -> PortResult<Observation> {
        let mut command = Command::new("git");
        command.arg("-C").arg(&self.root).arg("diff");
        if let Some(path) = args.get("path").and_then(Value::as_str) {
            command.arg("--").arg(path);
        }
        let output = command
            .output()
            .map_err(|error| PortError(format!("git diff: {error}")))?;
        if !output.status.success() {
            return Err(PortError(format!(
                "git diff: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        let text = String::from_utf8_lossy(&output.stdout).to_string();
        let mut output_text = text.chars().take(SHELL_OUTPUT_CAP).collect::<String>();
        if output_text.chars().count() >= SHELL_OUTPUT_CAP {
            output_text.push_str("\n[truncated at 64KB]");
        }
        Ok(Observation {
            name: "git_diff".to_string(),
            output: output_text,
        })
    }
}
