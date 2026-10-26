//! `serve`: build the runtime from the catalog and run the daemon.
//! `OREXIS_FAKE=1` uses deterministic fakes for tests.

use std::path::PathBuf;

use orexis_composition::{build_fake_runtime, build_runtime, BuildConfig};
use orexis_daemon::{Daemon, DaemonConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let socket = std::env::var("OREXIS_SOCKET")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp/orexis.sock"));
    let db = std::env::var("OREXIS_DB")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp/orexis.db"));
    let fake = std::env::var("OREXIS_FAKE").is_ok();

    let runtime = if fake {
        build_fake_runtime(db.clone())
    } else {
        let cwd = std::env::current_dir()?;
        let project_root = std::env::var("OREXIS_PROJECT")
            .map(PathBuf::from)
            .unwrap_or_else(|_| cwd.clone());
        build_runtime(BuildConfig {
            profile_id: std::env::var("OREXIS_PROFILE").unwrap_or_else(|_| "openai".to_string()),
            model_id: std::env::var("OREXIS_MODEL").unwrap_or_else(|_| "gpt-5".to_string()),
            db_path: db,
            project_root,
            cwd,
            providers_dir: None,
            system_prompt: String::new(),
            context_limit: 32768,
        })?
    };

    for warning in &runtime.warnings {
        eprintln!("warning: {warning}");
    }

    let config = DaemonConfig {
        socket,
        sessions: runtime.sessions,
        provider: runtime.provider,
        tools: runtime.tools,
        codec: runtime.codec,
        policy: runtime.policy,
        system_prompt: runtime.system_prompt,
        context_tools: runtime.context_tools,
        context_files: runtime.context_files,
        context_limit: runtime.context_limit,
        max_steps: if fake { 8 } else { 16 },
        generation_slots: runtime.generation_slots,
    };

    Daemon::new(config).run().await?;
    Ok(())
}
