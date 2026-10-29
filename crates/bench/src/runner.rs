//! The runner: one task, one mode, one daemon — measured from the
//! session log.

use std::io::Write;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use orexis_client::DaemonClient;
use orexis_composition::{build_runtime, BuildConfig};
use orexis_core::{BoardTask, SessionEvent, TokenUsage};
use orexis_daemon::{Daemon, DaemonConfig};
use orexis_protocol::ServerMsg;
use serde::Serialize;

use crate::task::BenchTask;

/// The execution mode a run measures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// One session, one worker — today's default.
    Serial,
    /// The session may decompose onto the board; workers claim tasks.
    Team,
}

impl Mode {
    pub fn name(self) -> &'static str {
        match self {
            Self::Serial => "serial",
            Self::Team => "team",
        }
    }
}

/// One row of the results JSONL: the metrics §10 asks for.
#[derive(Debug, Clone, Serialize)]
pub struct RunResult {
    pub task: String,
    pub mode: String,
    pub model: String,
    pub profile: String,
    pub held_out: bool,
    pub pass: bool,
    pub done: bool,
    /// Why the run stopped: done, error, or timeout.
    pub stop_reason: String,
    pub tool_calls: usize,
    pub model_calls: usize,
    pub workers: usize,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub wall_ms: u64,
    /// When the work actually finished: the board's drain time for team
    /// runs — the driver's turn may idle to the timeout after workers
    /// complete, so wall_ms overstates the team's real time (the gate's
    /// ≥25%-faster arm reads this, not wall_ms).
    pub work_ms: u64,
    /// Every session the run touched, driver first — workers included.
    /// The bench's failure diagnosis lives or dies on this list (M8-15
    /// showed a stop_reason alone can't say who failed).
    pub sessions: Vec<String>,
    /// `session_id: message` for every Error event in the run.
    pub errors: Vec<String>,
}

pub struct SuiteArgs {
    pub tasks: PathBuf,
    pub profile: String,
    pub model: String,
    pub modes: Vec<Mode>,
    /// Per-run wall clock cap; a run past it is a failed run, not a hang.
    pub timeout: Duration,
    pub out: PathBuf,
    /// Resume-safe output: append, never truncate.
    pub append: bool,
}

/// Run every (task, mode) pair once, writing one JSONL row per run.
pub async fn run_suite(args: &SuiteArgs) -> Result<(), Box<dyn std::error::Error>> {
    let tasks = crate::task::load(&args.tasks)?;
    let mut out = std::io::BufWriter::new(
        std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .append(args.append)
            .truncate(!args.append)
            .open(&args.out)?,
    );
    let total = tasks.len() * args.modes.len();
    let mut n = 0usize;
    for task in &tasks {
        for mode in &args.modes {
            n += 1;
            let result = match run_once(task, *mode, args).await {
                Ok(result) => result,
                Err(error) => RunResult {
                    task: task.name.clone(),
                    mode: mode.name().to_string(),
                    model: args.model.clone(),
                    profile: args.profile.clone(),
                    held_out: task.held_out,
                    pass: false,
                    done: false,
                    stop_reason: format!("error: {error}"),
                    tool_calls: 0,
                    model_calls: 0,
                    workers: 0,
                    prompt_tokens: 0,
                    completion_tokens: 0,
                    wall_ms: 0,
                    work_ms: 0,
                    sessions: Vec::new(),
                    errors: vec![format!("runner: {error}")],
                },
            };
            eprintln!(
                "[{n}/{total}] {} {} {}: pass={} stop={} wall={}ms tok={}+{}",
                task.name,
                mode.name(),
                args.model,
                result.pass,
                result.stop_reason,
                result.wall_ms,
                result.prompt_tokens,
                result.completion_tokens
            );
            writeln!(out, "{}", serde_json::to_string(&result)?)?;
            out.flush()?;
        }
    }
    eprintln!("wrote {}", args.out.display());
    Ok(())
}

/// One run: fixture workspace → daemon → prompt → wait → judge.
async fn run_once(
    task: &BenchTask,
    mode: Mode,
    args: &SuiteArgs,
) -> Result<RunResult, Box<dyn std::error::Error>> {
    let t0 = Instant::now();
    let state = tempfile::tempdir()?;
    // The fixture is the only thing the model's workspace holds; daemon
    // state lives beside it so the gate measures the task, not the daemon
    // (M8-16).
    let workspace = state.path().join("workspace");
    std::fs::create_dir(&workspace)?;
    for (path, content) in &task.fixture.files {
        let target = workspace.join(path);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&target, content)?;
    }

    let runtime = build_runtime(BuildConfig {
        profile_id: args.profile.clone(),
        model_id: args.model.clone(),
        db_path: state.path().join("bench.db"),
        project_root: workspace.clone(),
        cwd: workspace.clone(),
        providers_dir: None,
        mcp_dirs: Some(Vec::new()),
        utcp_dirs: Some(Vec::new()),
        a2a_dirs: Some(Vec::new()),
        evolution_key: None,
        verify_command: None,
        system_prompt: "You are a precise workspace agent. Complete the task using the \
                        tools, then finish with a summary."
            .to_string(),
        context_limit: 32768,
    })?;
    let sessions = runtime.sessions.clone();
    let socket = state.path().join("bench.sock");
    let config = DaemonConfig {
        socket: socket.clone(),
        profile_id: args.profile.clone(),
        model_id: args.model.clone(),
        sessions: runtime.sessions,
        entity: runtime.entity,
        identity_path: None,
        evolution: None,
        board: runtime.board,
        bus: runtime.bus,
        blackboard: runtime.blackboard,
        team: mode == Mode::Team,
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
        max_steps: 16,
        generation_slots: runtime.generation_slots,
    };
    let handle = tokio::spawn(Daemon::new(config).run());
    for _ in 0..50 {
        if socket.exists() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let mut client = DaemonClient::connect(&socket)?;
    let session_id = client.attach(None)?;
    client.prompt(&task.prompt, true)?;

    // Approval requests (any session's) get a yes: the bench measures
    // the harness, not the gate's latency. The deadline bounds the run.
    let deadline = t0 + args.timeout;
    let mut session_done = false;
    let mut error: Option<String> = None;
    let mut tasks: Vec<BoardTask> = Vec::new();
    let mut last_activity = Instant::now();
    let mut drained_at: Option<Instant> = None;
    let mut finished = false;
    while Instant::now() < deadline {
        if mode == Mode::Team {
            let _ = client.board();
        }
        while let Some(message) = client.try_recv()? {
            last_activity = Instant::now();
            match message {
                ServerMsg::ApprovalRequest { id, .. } => client.approve(id, true)?,
                ServerMsg::Event { event, .. } => match event {
                    SessionEvent::Done { .. } => session_done = true,
                    SessionEvent::Error { message } => {
                        session_done = true;
                        error = Some(message);
                    }
                    _ => {}
                },
                ServerMsg::Board { tasks: list } => tasks = list,
                _ => {}
            }
        }
        let drained = !tasks.is_empty()
            && tasks.iter().all(|task| {
                matches!(
                    task.status,
                    orexis_core::TaskStatus::Done | orexis_core::TaskStatus::Failed
                )
            });
        if drained && drained_at.is_none() {
            drained_at = Some(Instant::now());
        }
        finished = match mode {
            Mode::Serial => session_done,
            // A drained, quiet board is the team's finish line — the
            // driver's own turn may have ended without a Done event.
            Mode::Team => {
                session_done && drained
                    || drained && last_activity.elapsed() > Duration::from_secs(3)
            }
        };
        if finished {
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    let stop_reason = error.unwrap_or_else(|| {
        if finished {
            "done".to_string()
        } else {
            "timeout".to_string()
        }
    });
    let _ = client.detach();
    handle.abort();

    // Metrics: every session the run touched (the driver plus workers).
    let mut tool_calls = 0;
    let mut model_calls = 0;
    let mut workers = 0;
    let mut usage = TokenUsage::default();
    let mut done = false;
    let mut session_ids = Vec::new();
    let mut errors = Vec::new();
    for id in sessions.list()? {
        let refs = sessions.open(Some(&id))?;
        let events = refs.read.events().await?;
        if id != session_id && !events.is_empty() {
            workers += 1;
            session_ids.push(id.clone());
        }
        for stored in events {
            match stored.event {
                SessionEvent::ModelResponse { usage: u, .. } => {
                    model_calls += 1;
                    if let Some(u) = u {
                        usage.prompt += u.prompt;
                        usage.completion += u.completion;
                    }
                }
                SessionEvent::ToolCall { .. } => tool_calls += 1,
                SessionEvent::Done { .. } if id == session_id => done = true,
                SessionEvent::Error { message } => {
                    errors.push(format!("{id}: {message}"));
                }
                _ => {}
            }
        }
    }
    session_ids.insert(0, session_id);

    let mut pass = done || finished;
    for judge in &task.judges {
        pass &= judge.check(&workspace).map_err(|e| e.to_string())?;
    }
    // A run's evidence is its session log and board; the fixture workspace
    // is deleted on drop unless the caller asks for it (the M8 gate needed
    // it to tell a team pass from a driver pass).
    if std::env::var_os("OREXIS_BENCH_KEEP").is_some() {
        let kept = state.keep();
        eprintln!("kept workspace: {}", kept.display());
    }

    Ok(RunResult {
        task: task.name.clone(),
        mode: mode.name().to_string(),
        model: args.model.clone(),
        profile: args.profile.clone(),
        held_out: task.held_out,
        pass,
        done: finished,
        stop_reason,
        tool_calls,
        model_calls,
        workers,
        prompt_tokens: usage.prompt,
        completion_tokens: usage.completion,
        wall_ms: t0.elapsed().as_millis() as u64,
        work_ms: drained_at
            .unwrap_or(t0 + t0.elapsed())
            .duration_since(t0)
            .as_millis() as u64,
        sessions: session_ids,
        errors,
    })
}
