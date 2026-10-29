//! State, apply, palette, and search tests.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use orexis_adapters_tui::{Entry, Focus, Overlay, TuiAction, TuiApp, TuiState};
use orexis_core::SessionEvent;
use orexis_protocol::ServerMsg;
use ratatui::backend::TestBackend;
use ratatui::Terminal;

fn render(app: &TuiApp, width: u16, height: u16) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal.draw(|frame| app.render(frame)).expect("draw");
    let buffer = terminal.backend().buffer();
    (0..height)
        .map(|y| {
            (0..width)
                .map(|x| {
                    buffer
                        .cell(ratatui::layout::Position::new(x, y))
                        .map(|cell| cell.symbol())
                        .unwrap_or(" ")
                })
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[test]
fn enter_sends_the_composer() {
    let mut app = TuiApp::new();
    app.composer = "hello".to_string();
    assert_eq!(
        app.handle_key(key(KeyCode::Enter)),
        TuiAction::Send("hello".into())
    );
    assert!(app.composer.is_empty());
}

#[test]
fn shift_enter_newlines() {
    let mut app = TuiApp::new();
    app.composer = "one".to_string();
    let event = KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT);
    assert_eq!(app.handle_key(event), TuiAction::None);
    assert_eq!(app.composer, "one\n");
}

#[test]
fn esc_cancels() {
    let mut app = TuiApp::new();
    assert_eq!(app.handle_key(key(KeyCode::Esc)), TuiAction::Cancel);
}

#[test]
fn approval_y_approves() {
    let mut app = TuiApp::new();
    app.show_approval("rm -rf target", "high", false);
    assert_eq!(app.handle_key(key(KeyCode::Char('y'))), TuiAction::Approve);
}

#[test]
fn tab_toggles_focus() {
    let mut app = TuiApp::new();
    assert_eq!(app.focus, Focus::Composer);
    app.handle_key(key(KeyCode::Tab));
    assert_eq!(app.focus, Focus::Transcript);
    app.handle_key(key(KeyCode::Tab));
    assert_eq!(app.focus, Focus::Composer);
}

#[test]
fn tool_observation_pairs_with_the_call() {
    let mut app = TuiApp::new();
    app.push_event(&SessionEvent::ToolCall {
        call_id: "c1".into(),
        name: "read".into(),
        args: "src/lib.rs".into(),
    });
    app.push_event(&SessionEvent::Observation {
        call_id: "c1".into(),
        name: "read".into(),
        output: "ok".into(),
    });
    let Entry::Tool(row) = &app.entries[0] else {
        panic!("expected tool row");
    };
    assert!(row.ok);
    assert_eq!(row.detail, "ok");
}

#[test]
fn apply_maps_an_approval_request() {
    let mut app = TuiApp::new();
    app.apply(ServerMsg::ApprovalRequest {
        id: 7,
        session_id: "s1".to_string(),
        tool: "shell".to_string(),
        args: "ls".to_string(),
    });
    assert_eq!(app.pending_approval, Some(7));
    assert_eq!(app.state, TuiState::Approval);
}

#[test]
fn palette_filters_and_runs_a_command() {
    let mut app = TuiApp::new();
    app.overlay = Some(Overlay::Palette);
    app.focus = Focus::Overlay;
    app.palette_query = "undo".to_string();
    assert_eq!(app.handle_key(key(KeyCode::Enter)), TuiAction::Undo);
    assert!(app.overlay.is_none());
}

#[test]
fn search_overlay_renders_matches() {
    let mut app = TuiApp::new();
    app.push_event(&SessionEvent::UserMessage {
        text: "find the needle".to_string(),
    });
    app.overlay = Some(Overlay::Search);
    app.focus = Focus::Overlay;
    app.search_query = "needle".to_string();
    let rendered = render(&app, 80, 24);
    assert!(rendered.contains("needle"));
}

#[test]
fn same_name_tools_pair_by_call_id() {
    let mut app = TuiApp::new();
    app.push_event(&SessionEvent::ToolCall {
        call_id: "c1".to_string(),
        name: "read_file".to_string(),
        args: "a".to_string(),
    });
    app.push_event(&SessionEvent::ToolCall {
        call_id: "c2".to_string(),
        name: "read_file".to_string(),
        args: "b".to_string(),
    });
    app.push_event(&SessionEvent::Observation {
        call_id: "c2".to_string(),
        name: "read_file".to_string(),
        output: "second".to_string(),
    });
    let Entry::Tool(first) = &app.entries[0] else {
        panic!("expected tool");
    };
    let Entry::Tool(second) = &app.entries[1] else {
        panic!("expected tool");
    };
    assert!(!first.ok);
    assert!(second.ok);
    assert_eq!(second.detail, "second");
}

#[test]
fn delta_stream_appends_and_final_clears() {
    let mut app = TuiApp::new();
    app.apply(ServerMsg::Delta {
        text: "he".to_string(),
    });
    app.apply(ServerMsg::Delta {
        text: "llo".to_string(),
    });
    assert_eq!(app.live.as_deref(), Some("hello"));
    assert!(render(&app, 80, 24).contains("hello"));

    app.push_event(&SessionEvent::ModelResponse {
        text: "hello".to_string(),
        actions: Vec::new(),
        usage: None,
    });
    assert!(app.live.is_none());
    assert!(app
        .entries
        .iter()
        .any(|entry| matches!(entry, Entry::Assistant(text) if text == "hello")));
}

#[test]
fn error_clears_live() {
    let mut app = TuiApp::new();
    app.apply(ServerMsg::Delta {
        text: "hi".to_string(),
    });
    app.push_event(&SessionEvent::Error {
        message: "boom".to_string(),
    });
    assert!(app.live.is_none());
}

#[test]
fn self_view_sets_overlay() {
    let mut app = TuiApp::new();
    app.apply(ServerMsg::SelfView {
        text: "drives connection 0.5".to_string(),
    });
    assert_eq!(app.self_text.as_deref(), Some("drives connection 0.5"));
    assert!(matches!(app.overlay, Some(Overlay::SelfState)));
}

#[test]
fn board_sets_overlay_and_renders_tasks() {
    let mut app = TuiApp::new();
    app.apply(ServerMsg::Board {
        tasks: vec![orexis_core::BoardTask {
            id: "task-1".to_string(),
            title: "fix the bug".to_string(),
            required_strategy: "build".to_string(),
            owner: Some("worker-task-1".to_string()),
            status: orexis_core::TaskStatus::Running,
            depends_on: vec![],
            seq: 1,
        }],
    });
    let text = app.board_text.as_deref().expect("board text");
    assert!(text.contains("fix the bug"));
    assert!(text.contains("running"));
    assert!(text.contains("build"));
    assert!(matches!(app.overlay, Some(Overlay::Board)));

    // An empty board says so instead of rendering nothing.
    let mut app = TuiApp::new();
    app.apply(ServerMsg::Board { tasks: vec![] });
    assert_eq!(app.board_text.as_deref(), Some("no board tasks"));
}

#[test]
fn fleet_sets_overlay_and_renders_workers() {
    let mut app = TuiApp::new();
    app.apply(ServerMsg::Fleet {
        workers: vec![orexis_core::BoardTask {
            id: "task-1".to_string(),
            title: "fix the bug".to_string(),
            required_strategy: "build".to_string(),
            owner: Some("worker-task-1".to_string()),
            status: orexis_core::TaskStatus::Running,
            depends_on: vec![],
            seq: 1,
        }],
    });
    let text = app.fleet_text.as_deref().expect("fleet text");
    assert!(text.contains("worker-task-1"));
    assert!(text.contains("running"));
    assert!(text.contains("fix the bug"));
    assert!(matches!(app.overlay, Some(Overlay::Fleet)));

    let mut app = TuiApp::new();
    app.apply(ServerMsg::Fleet { workers: vec![] });
    assert_eq!(app.fleet_text.as_deref(), Some("no live workers"));
}

#[test]
fn artifacts_set_overlay() {
    let mut app = TuiApp::new();
    app.apply(ServerMsg::Artifacts {
        items: vec![orexis_protocol::Artifact {
            id: "notes.md".to_string(),
            kind: "artifact".to_string(),
            title: "notes.md".to_string(),
            detail: String::new(),
        }],
    });
    let text = app.artifacts_text.as_deref().expect("artifacts text");
    assert!(text.contains("notes.md"));
    assert!(matches!(app.overlay, Some(Overlay::Artifacts)));
}

#[test]
fn evolution_sets_overlay() {
    let mut app = TuiApp::new();
    app.apply(ServerMsg::Evolution {
        entries: vec!["#1 apply".to_string()],
    });
    assert_eq!(app.evolution_text.as_deref(), Some("#1 apply"));
    assert!(matches!(app.overlay, Some(Overlay::Evolution)));
}

#[test]
fn done_and_interrupted_transition_state_and_extract_readable_tool_target() {
    let mut app = TuiApp::new();
    app.push_event(&SessionEvent::ToolCall {
        call_id: "c1".into(),
        name: "read_file".into(),
        args: r#"{"path":"src/main.rs","extra":1}"#.into(),
    });
    let Entry::Tool(row) = &app.entries[0] else {
        panic!("expected tool row");
    };
    assert_eq!(row.target, "src/main.rs");

    app.push_event(&SessionEvent::Done {
        summary: "all good".into(),
    });
    assert_eq!(app.state, TuiState::Empty);

    app.push_event(&SessionEvent::UserMessage {
        text: "again".into(),
    });
    assert_eq!(app.state, TuiState::Streaming);
    app.push_event(&SessionEvent::Interrupted {
        reason: "budget exceeded: 100 > 50".into(),
    });
    assert_eq!(app.state, TuiState::BudgetExhausted);
}

#[test]
fn diff_message_populates_bottom_sheet_and_transcript_j_k_moves_selected() {
    let mut app = TuiApp::new();
    app.apply(ServerMsg::Diff {
        diff: "--- a/x\n+++ b/x\n".into(),
    });
    assert_eq!(app.bottom_sheet.as_deref(), Some("--- a/x\n+++ b/x\n"));

    app.push_event(&SessionEvent::ToolCall {
        call_id: "c1".into(),
        name: "read_file".into(),
        args: r#"{"path":"a.rs"}"#.into(),
    });
    app.push_event(&SessionEvent::ToolCall {
        call_id: "c2".into(),
        name: "read_file".into(),
        args: r#"{"path":"b.rs"}"#.into(),
    });
    app.focus = Focus::Transcript;
    assert_eq!(app.selected, 0);
    app.handle_key(key(KeyCode::Char('j')));
    assert_eq!(app.selected, 1);
    assert_eq!(
        app.handle_key(key(KeyCode::Char('o'))),
        TuiAction::OpenFile("b.rs".into())
    );
}

#[test]
fn composer_history_navigates_up_and_down_preserving_draft() {
    let mut app = TuiApp::new();
    app.composer = "first".into();
    assert_eq!(
        app.handle_key(key(KeyCode::Enter)),
        TuiAction::Send("first".into())
    );
    app.composer = "second".into();
    assert_eq!(
        app.handle_key(key(KeyCode::Enter)),
        TuiAction::Send("second".into())
    );

    app.composer = "draft".into();
    app.handle_key(key(KeyCode::Up));
    assert_eq!(app.composer, "second");
    app.handle_key(key(KeyCode::Up));
    assert_eq!(app.composer, "first");
    app.handle_key(key(KeyCode::Down));
    assert_eq!(app.composer, "second");
    app.handle_key(key(KeyCode::Down));
    assert_eq!(app.composer, "draft");
}

#[test]
fn streaming_auto_scrolls_to_bottom_and_unlocks_on_scroll_up() {
    let mut app = TuiApp::new();
    assert!(app.auto_scroll);
    for i in 0..30 {
        app.push_event(&SessionEvent::UserMessage {
            text: format!("msg-{i:02}"),
        });
    }
    let out = render(&app, 80, 12);
    assert!(
        out.contains("msg-29"),
        "auto_scroll should show tail: {out}"
    );
    assert!(!out.contains("msg-00"));

    app.focus = Focus::Transcript;
    app.handle_key(key(KeyCode::Char('g')));
    assert!(!app.auto_scroll);
    let top_out = render(&app, 80, 12);
    assert!(top_out.contains("msg-00"));

    app.handle_key(key(KeyCode::Char('G')));
    assert!(app.auto_scroll);
    let tail_out = render(&app, 80, 12);
    assert!(tail_out.contains("msg-29"));
}
