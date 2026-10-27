//! Runtime construction from the environment, shared by the bins.

use std::path::PathBuf;

use orexis_composition::{build_fake_runtime, build_runtime, BuildConfig};

use crate::server::DaemonConfig;

/// Build the daemon config from the `OREXIS_*` environment variables.
/// The socket is serve-specific; the ACP server ignores it.
pub fn daemon_config_from_env() -> Result<DaemonConfig, Box<dyn std::error::Error>> {
    let socket = std::env::var("OREXIS_SOCKET")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp/orexis.sock"));
    let db = std::env::var("OREXIS_DB")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp/orexis.db"));
    let fake = std::env::var("OREXIS_FAKE").is_ok();
    let profile = std::env::var("OREXIS_PROFILE").unwrap_or_else(|_| "openai".to_string());
    let model = std::env::var("OREXIS_MODEL").unwrap_or_else(|_| "gpt-5".to_string());

    // Fake mode uses neither the workspace nor the project layout; the
    // current dir is also a failure path fake mode must not gain.
    let (runtime, identity_path) = if fake {
        (build_fake_runtime(db.clone()), None)
    } else {
        let cwd = std::env::current_dir()?;
        let project_root = std::env::var("OREXIS_PROJECT")
            .map(PathBuf::from)
            .unwrap_or_else(|_| cwd.clone());
        let identity_path = Some(project_root.join(".orexis/IDENTITY.md"));
        let runtime = build_runtime(BuildConfig {
            profile_id: profile.clone(),
            model_id: model.clone(),
            db_path: db,
            project_root,
            cwd,
            providers_dir: None,
            mcp_dirs: None,
            utcp_dirs: None,
            a2a_dirs: None,
            evolution_key: std::env::var("OREXIS_EVOLUTION_KEY").ok(),
            verify_command: std::env::var("OREXIS_VERIFY").ok(),
            system_prompt: String::new(),
            context_limit: 32768,
        })?;
        (runtime, identity_path)
    };

    for warning in &runtime.warnings {
        eprintln!("warning: {warning}");
    }

    Ok(DaemonConfig {
        socket,
        profile_id: if fake { "fake".to_string() } else { profile },
        model_id: if fake { "fake".to_string() } else { model },
        sessions: runtime.sessions,
        entity: runtime.entity,
        identity_path,
        evolution: runtime.evolution,
        external: runtime.external,
        provider: runtime.provider,
        tools: runtime.tools,
        workspace: runtime.workspace,
        codec: runtime.codec,
        policy: runtime.policy,
        checkpointer: runtime.checkpointer,
        verifier: runtime.verifier,
        system_prompt: runtime.system_prompt,
        context_tools: runtime.context_tools,
        context_files: runtime.context_files,
        context_limit: runtime.context_limit,
        max_steps: if fake { 8 } else { 16 },
        generation_slots: runtime.generation_slots,
    })
}
