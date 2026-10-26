//! `serve`: build the runtime from the catalog and run the daemon.
//! `OREXIS_FAKE=1` uses deterministic fakes for tests.

use std::path::PathBuf;
use std::sync::Arc;

use orexis_adapters_codec::NativeCodec;
use orexis_adapters_fake::{EchoTools, ScriptedProvider};
use orexis_composition::{build_runtime, BuildConfig};
use orexis_core::Action;
use orexis_daemon::{Daemon, DaemonConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let socket = std::env::var("OREXIS_SOCKET")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp/orexis.sock"));
    let db = std::env::var("OREXIS_DB")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp/orexis.db"));

    let config = if std::env::var("OREXIS_FAKE").is_ok() {
        DaemonConfig {
            socket,
            db,
            provider: Arc::new(ScriptedProvider::new(vec![vec![Action::Done {
                summary: "fake provider".to_string(),
            }]])),
            tools: Arc::new(EchoTools),
            codec: Arc::new(NativeCodec),
            system_prompt: String::new(),
            context_tools: Vec::new(),
            context_files: Vec::new(),
            context_limit: 32768,
            max_steps: 8,
            generation_slots: 1,
        }
    } else {
        let cwd = std::env::current_dir()?;
        let project_root = std::env::var("OREXIS_PROJECT")
            .map(PathBuf::from)
            .unwrap_or_else(|_| cwd.clone());
        let runtime = build_runtime(BuildConfig {
            profile_id: std::env::var("OREXIS_PROFILE").unwrap_or_else(|_| "openai".to_string()),
            model_id: std::env::var("OREXIS_MODEL").unwrap_or_else(|_| "gpt-5".to_string()),
            db_path: db.clone(),
            project_root,
            cwd,
            system_prompt: String::new(),
            context_limit: 32768,
        })?;
        DaemonConfig {
            socket,
            db,
            provider: runtime.provider,
            tools: runtime.tools,
            codec: runtime.codec,
            system_prompt: runtime.system_prompt,
            context_tools: runtime.context_tools,
            context_files: runtime.context_files,
            context_limit: runtime.context_limit,
            max_steps: 16,
            generation_slots: 1,
        }
    };

    Daemon::new(config).run().await?;
    Ok(())
}
