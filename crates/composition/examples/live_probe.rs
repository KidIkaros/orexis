//! Live provider probe: dry connectivity or a small tool-call turn.
//!
//! Env:
//!   OREXIS_PROFILE  profile id (openai, anthropic, llamacpp, or a custom profile)
//!   OREXIS_MODEL    model id
//!   OREXIS_DRY      "1" for a connectivity-only request (no tools)
//!   OREXIS_PROMPT   optional prompt
//!
//! Secrets resolve through the keyring first (`secret-tool`), then env.

use orexis_composition::{build_runtime, BuildConfig};
use orexis_core::{Message, Role, SessionEvent};
use orexis_ports::{EventSink, SessionRefs};
use orexis_usecases::{run_turn, ContextConfig, TurnDeps};

struct PrintSink;

impl EventSink for PrintSink {
    fn emit(&self, event: &SessionEvent, _seq: u64) {
        match event {
            SessionEvent::ModelResponse { text, .. } if !text.is_empty() => {
                println!("model: {text}");
            }
            SessionEvent::ToolCall { name, args, .. } => {
                println!("tool call: {name} {args}");
            }
            SessionEvent::Observation { name, output, .. } => {
                let short: String = output.chars().take(200).collect();
                println!("observation: {name}: {short}");
            }
            SessionEvent::Done { summary } => println!("done: {summary}"),
            SessionEvent::Error { message } => println!("error: {message}"),
            SessionEvent::Verify { passed, .. } => println!("verify: {passed}"),
            SessionEvent::Reprompt { note } => println!("reprompt: {note}"),
            _ => {}
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let profile = std::env::var("OREXIS_PROFILE").unwrap_or_else(|_| "openai".to_string());
    let model = std::env::var("OREXIS_MODEL")?;
    let dry = std::env::var("OREXIS_DRY")
        .map(|value| value != "0")
        .unwrap_or(false);
    let repo = std::env::var("OREXIS_REPO")
        .map(|value| value != "0")
        .unwrap_or(false);
    let prompt = std::env::var("OREXIS_PROMPT").unwrap_or_else(|_| {
        if dry {
            "Reply with exactly the word ok and nothing else.".to_string()
        } else if repo {
            "In this repository, src/lib.rs has a bug: add subtracts instead of adds. Read src/lib.rs, edit `a - b` to `a + b`, run `cargo test`, then finish with a summary.".to_string()
        } else {
            "Use read_file to read hello.txt, then finish with a one-line summary.".to_string()
        }
    });

    let dir = tempfile::tempdir()?;
    if repo {
        setup_repo(dir.path())?;
    } else {
        std::fs::write(dir.path().join("hello.txt"), "hello from orexis\n")?;
    }

    let runtime = build_runtime(BuildConfig {
        profile_id: profile,
        model_id: model,
        db_path: dir.path().join("probe.db"),
        project_root: dir.path().to_path_buf(),
        cwd: dir.path().to_path_buf(),
        providers_dir: None,
        verify_command: std::env::var("OREXIS_VERIFY").ok(),
        system_prompt: "You are a coding agent. Use the provided tools.".to_string(),
        context_limit: 32768,
    })?;

    for warning in &runtime.warnings {
        eprintln!("warning: {warning}");
    }

    if dry {
        let messages = vec![Message {
            role: Role::User,
            text: prompt,
            tool_call_id: None,
            tool_calls: Vec::new(),
        }];
        let response = runtime.provider.complete(&messages, &[], 0).await?;
        for action in response.actions {
            println!("{action:?}");
        }
        return Ok(());
    }

    let refs: SessionRefs = runtime.sessions.open(None)?;
    let context = ContextConfig {
        system_prompt: &runtime.system_prompt,
        tools: &runtime.context_tools,
        context_files: &runtime.context_files,
        context_limit: runtime.context_limit,
    };
    let sink = PrintSink;
    let outcome = run_turn(TurnDeps {
        provider: runtime.provider.as_ref(),
        tools: runtime.tools.as_ref(),
        session: refs.append.as_ref(),
        reader: refs.read.as_ref(),
        sink: &sink,
        session_id: &refs.id,
        prompt: &prompt,
        max_steps: if repo { 8 } else { 4 },
        codec: runtime.codec.as_ref(),
        checkpointer: runtime.checkpointer.as_deref(),
        verifier: runtime.verifier.as_deref(),
        skip_verify: runtime.verifier.is_none(),
        delta_sink: &orexis_ports::NullDeltaSink,
        context,
    })
    .await?;
    println!("steps: {} done: {}", outcome.steps, outcome.done);
    Ok(())
}

fn setup_repo(root: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(root.join("src"))?;
    std::fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )?;
    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn add(a: i32, b: i32) -> i32 { a - b }\n\n#[test]\nfn adds() { assert_eq!(add(2, 3), 5); }\n",
    )?;

    let git = |args: &[&str]| -> Result<(), Box<dyn std::error::Error>> {
        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_NAME", "probe")
            .env("GIT_AUTHOR_EMAIL", "probe@localhost")
            .env("GIT_COMMITTER_NAME", "probe")
            .env("GIT_COMMITTER_EMAIL", "probe@localhost")
            .output()?;
        if !output.status.success() {
            return Err(format!(
                "git {:?}: {}",
                args,
                String::from_utf8_lossy(&output.stderr)
            )
            .into());
        }
        Ok(())
    };
    git(&["init", "-q"])?;
    git(&["add", "-A"])?;
    git(&["commit", "-q", "-m", "init"])?;
    Ok(())
}
