//! Rendering tests for every state and overlay.

use orexis_adapters_tui::render::draw;
use orexis_adapters_tui::{Focus, Overlay, PlanTask, TuiApp, TuiState};
use orexis_core::SessionEvent;
use ratatui::backend::TestBackend;
use ratatui::Terminal;

fn render(app: &TuiApp, width: u16, height: u16) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal.draw(|frame| draw(frame, app)).expect("draw");
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

#[test]
fn empty_state_shows_init_hint() {
    let app = TuiApp::new();
    assert!(render(&app, 80, 24).contains("no model configured"));
}

#[test]
fn streaming_state_renders_transcript() {
    let mut app = TuiApp::new();
    app.push_event(&SessionEvent::UserMessage {
        text: "refactor the store".into(),
    });
    app.push_event(&SessionEvent::ModelResponse {
        text: "I'll read it first.".into(),
        actions: Vec::new(),
    });
    let out = render(&app, 80, 24);
    assert!(out.contains("you › refactor the store"));
    assert!(out.contains("orexis › I'll read it first."));
}

#[test]
fn tool_row_renders_and_expands() {
    let mut app = TuiApp::new();
    app.push_event(&SessionEvent::ToolCall {
        call_id: "c1".into(),
        name: "read".into(),
        args: "src/lib.rs".into(),
    });
    app.push_event(&SessionEvent::Observation {
        call_id: "c1".into(),
        name: "read".into(),
        output: "fn main() {}".into(),
    });
    assert!(render(&app, 80, 24).contains("read"));

    app.focus = Focus::Transcript;
    app.selected = 0;
    app.handle_key(crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Enter,
        crossterm::event::KeyModifiers::NONE,
    ));
    assert!(render(&app, 80, 24).contains("fn main() {}"));
}

#[test]
fn approval_card_renders() {
    let mut app = TuiApp::new();
    app.show_approval("rm -rf target", "high", false);
    let out = render(&app, 80, 24);
    assert!(out.contains("approval"));
    assert!(out.contains("rm -rf target"));
    assert!(out.contains("[y] approve"));
}

#[test]
fn plan_card_renders() {
    let mut app = TuiApp::new();
    app.show_plan(vec![
        PlanTask {
            text: "read store".into(),
            done: true,
            current: false,
        },
        PlanTask {
            text: "rewrite append".into(),
            done: false,
            current: true,
        },
    ]);
    let out = render(&app, 80, 24);
    assert!(out.contains("plan — 2 tasks"));
    assert!(out.contains("read store"));
    assert!(out.contains("rewrite append"));
}

#[test]
fn error_state_renders() {
    let mut app = TuiApp::new();
    app.set_error("provider boom");
    assert!(render(&app, 80, 24).contains("error: provider boom"));
}

#[test]
fn budget_state_renders() {
    let mut app = TuiApp::new();
    app.set_budget(42_000, 100_000);
    assert!(render(&app, 80, 24).contains("budget exhausted: 42000/100000"));
}

#[test]
fn disconnected_state_renders() {
    let mut app = TuiApp::new();
    app.set_state(TuiState::Disconnected);
    assert!(render(&app, 80, 24).contains("disconnected"));
}

#[test]
fn queued_state_renders() {
    let mut app = TuiApp::new();
    app.set_queued(2);
    assert!(render(&app, 80, 24).contains("queued: position 2"));
}

#[test]
fn compacting_state_renders() {
    let mut app = TuiApp::new();
    app.set_state(TuiState::Compacting);
    assert!(render(&app, 80, 24).contains("compacting context"));
}

#[test]
fn status_bar_compacts_at_80_and_expands_at_120() {
    let app = TuiApp::new();
    let narrow = render(&app, 80, 24);
    assert!(narrow.contains("[empty] s1 · 1/1 · 0%"));
    let wide = render(&app, 120, 40);
    assert!(wide.contains("worker 1/1"));
    assert!(wide.contains("plan:off"));
}

#[test]
fn all_states_render_at_both_sizes() {
    let states = [
        TuiState::Empty,
        TuiState::Loading,
        TuiState::Streaming,
        TuiState::ToolRunning,
        TuiState::Approval,
        TuiState::PlanGate,
        TuiState::Error,
        TuiState::BudgetExhausted,
        TuiState::Disconnected,
        TuiState::Queued,
        TuiState::Compacting,
    ];
    for (width, height) in [(80, 24), (120, 40)] {
        for state in states {
            let mut app = TuiApp::new();
            app.set_state(state);
            let out = render(&app, width, height);
            assert!(!out.trim().is_empty(), "{state:?} at {width}x{height}");
        }
    }
}

#[test]
fn no_color_still_renders_text() {
    let mut app = TuiApp::new();
    app.color = false;
    app.push_event(&SessionEvent::UserMessage {
        text: "hi".to_string(),
    });
    assert!(render(&app, 80, 24).contains("you › hi"));
}

#[test]
fn fleet_overlay_renders() {
    let mut app = TuiApp::new();
    app.apply(orexis_protocol::ServerMsg::Fleet {
        workers: vec![orexis_core::BoardTask {
            id: "task-12".to_string(),
            title: "rewrite store".to_string(),
            required_strategy: "build".to_string(),
            owner: Some("worker-task-12".to_string()),
            status: orexis_core::TaskStatus::Running,
            depends_on: vec![],
            seq: 12,
        }],
    });
    app.focus = Focus::Overlay;
    let out = render(&app, 120, 40);
    assert!(out.contains("fleet"));
    assert!(out.contains("worker-task-12"));
    assert!(out.contains("running"));
}
