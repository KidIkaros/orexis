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
