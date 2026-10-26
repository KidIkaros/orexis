//! Rendering for the TUI (ADR-0004).

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::{Entry, Focus, Overlay, TuiApp, TuiState};

pub fn draw(frame: &mut Frame, app: &TuiApp) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(area);

    let status_style = if app.color {
        Style::default().add_modifier(Modifier::REVERSED)
    } else {
        Style::default()
    };
    frame.render_widget(
        Paragraph::new(app.status_line(area.width)).style(status_style),
        chunks[0],
    );

    let lines = transcript_lines(app);
    let height = chunks[1].height as usize;
    let start = app.scroll.min(lines.len().saturating_sub(height));
    let visible: Vec<Line> = lines.into_iter().skip(start).take(height).collect();
    if visible.is_empty() {
        frame.render_widget(
            Paragraph::new(state_message(app))
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: false }),
            chunks[1],
        );
    } else {
        frame.render_widget(
            Paragraph::new(visible).wrap(Wrap { trim: false }),
            chunks[1],
        );
    }

    frame.render_widget(
        Paragraph::new(format!("› {}", app.composer)).block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(dim(app)),
        ),
        chunks[2],
    );

    frame.render_widget(Paragraph::new(app.keyhints()).style(dim(app)), chunks[3]);

    if let Some(overlay) = app.overlay {
        draw_overlay(frame, overlay, area);
    }
    if let Some(sheet) = &app.bottom_sheet {
        draw_bottom_sheet(frame, sheet, chunks[1]);
    }

    if app.focus == Focus::Composer {
        let x = chunks[2].x + 2 + app.composer.chars().count() as u16;
        let x = x.min(chunks[2].right().saturating_sub(1));
        let y = chunks[2].y + 1;
        let y = y.min(chunks[2].bottom().saturating_sub(1));
        frame.set_cursor_position((x, y));
    }
}

fn bold(app: &TuiApp) -> Style {
    if app.color {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    }
}

fn dim(app: &TuiApp) -> Style {
    if app.color {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default()
    }
}

fn transcript_lines(app: &TuiApp) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    for entry in &app.entries {
        match entry {
            Entry::User(text) => lines.push(Line::from(vec![
                Span::styled("you › ", bold(app)),
                Span::raw(text.clone()),
            ])),
            Entry::Assistant(text) => lines.push(Line::from(vec![
                Span::styled("orexis › ", bold(app)),
                Span::raw(text.clone()),
            ])),
            Entry::Tool(row) => {
                let glyph = if row.expanded { "▾" } else { "▸" };
                let status = if row.ok { "✓" } else { "…" };
                lines.push(Line::from(format!(
                    "{glyph} {:<6} {:<28} {:>5.1}s {status}",
                    row.name,
                    row.target,
                    row.elapsed_ms as f64 / 1000.0
                )));
                if row.expanded {
                    for detail in row.detail.lines() {
                        lines.push(Line::from(format!("  {detail}")));
                    }
                }
            }
            Entry::Approval {
                command,
                risk,
                reversible,
            } => {
                lines.push(Line::from(format!("⚠ approval  shell  {command}")));
                lines.push(Line::from(format!(
                    "  risk: {risk}   reversible: {}",
                    if *reversible { "yes" } else { "no" }
                )));
                lines.push(Line::from(
                    "  [y] approve   [n] deny   [d] diff   [a] always allow",
                ));
            }
            Entry::Plan { tasks } => {
                lines.push(Line::from(format!("plan — {} tasks", tasks.len())));
                for task in tasks {
                    let glyph = if task.done {
                        "✓"
                    } else if task.current {
                        "▶"
                    } else {
                        "○"
                    };
                    lines.push(Line::from(format!("  {glyph} {}", task.text)));
                }
                lines.push(Line::from("  [a]pprove   [e]dit   [c]ancel"));
            }
            Entry::Error(message) => lines.push(Line::from(Span::styled(
                format!("✗ {message}"),
                Style::default().fg(Color::Red),
            ))),
            Entry::Info(message) => {
                lines.push(Line::from(Span::styled(format!("· {message}"), dim(app))))
            }
        }
    }
    lines
}

fn state_message(app: &TuiApp) -> String {
    match app.state {
        TuiState::Empty => "no model configured — run `orexis init`".to_string(),
        TuiState::Loading => "starting provider…".to_string(),
        TuiState::Error => format!(
            "error: {}",
            app.error.clone().unwrap_or_else(|| "unknown".to_string())
        ),
        TuiState::BudgetExhausted => app
            .budget
            .map(|(spent, limit)| format!("budget exhausted: {spent}/{limit} tokens"))
            .unwrap_or_else(|| "budget exhausted".to_string()),
        TuiState::Disconnected => "disconnected — reattach with `orexis attach`".to_string(),
        TuiState::Queued => app
            .queued
            .map(|position| format!("queued: position {position}"))
            .unwrap_or_else(|| "queued".to_string()),
        TuiState::Compacting => "compacting context…".to_string(),
        _ => String::new(),
    }
}

fn draw_overlay(frame: &mut Frame, overlay: Overlay, area: Rect) {
    let rect = if area.width >= 120 {
        Rect {
            x: area.width.saturating_sub(40),
            y: 1,
            width: 40,
            height: area.height.saturating_sub(2),
        }
    } else {
        Rect {
            x: 0,
            y: 1,
            width: area.width,
            height: area.height.saturating_sub(2),
        }
    };
    frame.render_widget(Clear, rect);
    frame.render_widget(
        Paragraph::new(overlay_lines(overlay))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(overlay.title()),
            )
            .wrap(Wrap { trim: false }),
        rect,
    );
}

fn overlay_lines(overlay: Overlay) -> Vec<Line<'static>> {
    match overlay {
        Overlay::Fleet => vec![
            Line::from("● 1  builder   claude-sonnet   running  task 12   $0.04"),
            Line::from("● 2  reviewer  gpt-5           waiting  task 12   $0.01"),
            Line::from("○ 3  planner   local 8b        idle               $0.00"),
            Line::from("○ 4  miner     deepseek        queued   task 14   $0.00"),
        ],
        Overlay::Board => vec![
            Line::from("#12  rewrite store     build    worker 1   running"),
            Line::from("#13  review store      review   worker 2   blocked by #12"),
            Line::from("#14  migrate tests     build    —          queued"),
        ],
        Overlay::SelfState => vec![
            Line::from("drives   curiosity 0.7 · mastery 0.6 · coherence 0.8 · rest 0.3"),
            Line::from("affect   valence +0.2 · arousal 0.4 · dominance 0.5"),
            Line::from("tier     skills        budget 42k/100k    provider anthropic"),
            Line::from("leaves   prompts + tool args + file reads → api.anthropic.com"),
        ],
        Overlay::Help => vec![
            Line::from("Enter send · ⇧Enter newline · Esc cancel"),
            Line::from("Ctrl-C interrupt · Ctrl-D detach · Ctrl-F fleet"),
            Line::from("Ctrl-B board · ? help · Tab focus"),
            Line::from("j/k scroll · Enter expand · o open · y copy"),
            Line::from("approval: y/n/d/a · plan: a/e/c"),
        ],
    }
}

fn draw_bottom_sheet(frame: &mut Frame, sheet: &str, area: Rect) {
    let height = (area.height / 3).max(5);
    let rect = Rect {
        x: area.x,
        y: area.bottom().saturating_sub(height),
        width: area.width,
        height,
    };
    frame.render_widget(Clear, rect);
    frame.render_widget(
        Paragraph::new(sheet.to_string())
            .block(Block::default().borders(Borders::ALL).title("detail"))
            .wrap(Wrap { trim: false }),
        rect,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PlanTask;
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
        app.overlay = Some(Overlay::Fleet);
        app.focus = Focus::Overlay;
        let out = render(&app, 120, 40);
        assert!(out.contains("fleet"));
        assert!(out.contains("builder"));
    }
}
