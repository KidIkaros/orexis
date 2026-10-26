//! Git and hex helpers.

use std::path::Path;
use std::process::Command;

use orexis_ports::{PortError, PortResult};

pub(crate) fn run_git(root: &Path, args: &[&str]) -> PortResult<()> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(|error| PortError(format!("git: {error}")))?;
    if !output.status.success() {
        return Err(PortError(format!(
            "git {:?}: {}",
            args,
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(())
}

pub(crate) fn git_output(root: &Path, args: &[&str]) -> PortResult<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(|error| PortError(format!("git: {error}")))?;
    if !output.status.success() {
        return Err(PortError(format!(
            "git {:?}: {}",
            args,
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub(crate) fn hex_decode(text: &str) -> PortResult<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return Err(PortError("odd hex length".into()));
    }
    (0..text.len())
        .step_by(2)
        .map(|index| {
            u8::from_str_radix(&text[index..index + 2], 16)
                .map_err(|error| PortError(format!("hex: {error}")))
        })
        .collect()
}
