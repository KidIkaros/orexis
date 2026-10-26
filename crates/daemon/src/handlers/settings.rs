//! Settings handler.

use std::sync::Arc;

use tokio::net::unix::OwnedWriteHalf;

use crate::proto::{write_msg, ServerMsg, SettingsInfo};
use crate::server::DaemonConfig;

pub(crate) async fn settings(
    config: &Arc<DaemonConfig>,
    write: &mut OwnedWriteHalf,
) -> std::io::Result<()> {
    let permissions = [
        "read_file",
        "write_file",
        "edit_file",
        "shell",
        "git_commit",
        "verify",
    ]
    .into_iter()
    .map(|tool| format!("{tool}: {:?}", config.policy.decide(tool)).to_lowercase())
    .collect();
    let privacy = vec![
        format!("provider: {} ({})", config.profile_id, config.model_id),
        "prompts and tool results go to the configured provider".to_string(),
        "no telemetry; no remote UI; no HTTP server".to_string(),
        "keys: OS keyring first, then environment".to_string(),
        "sessions: local SQLite; checkpoints: local .orexis".to_string(),
    ];
    write_msg(
        write,
        &ServerMsg::Settings {
            settings: SettingsInfo {
                profile: config.profile_id.clone(),
                model: config.model_id.clone(),
                plan_mode: config.policy.plan_mode,
                permissions,
                privacy,
            },
        },
    )
    .await
}
