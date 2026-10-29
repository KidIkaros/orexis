//! Rendering for the TUI (ADR-0004).

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::{Entry, Focus, TuiApp, TuiState};

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
    let max_scroll = lines.len().saturating_sub(height);
    let start = if app.auto_scroll {
        max_scroll
    } else {
        app.scroll.min(max_scroll)
    };
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
        crate::overlays::draw_overlay(frame, app, overlay, area);
    }
    if let Some(sheet) = &app.bottom_sheet {
        crate::overlays::draw_bottom_sheet(frame, app, sheet, chunks[1]);
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

pub(crate) fn transcript_lines(app: &TuiApp) -> Vec<Line<'static>> {
    let mut cache = app.cache.borrow_mut();
    if cache.revision != app.revision {
        cache.lines = build_transcript_lines(app);
        cache.revision = app.revision;
    }
    cache.lines.clone()
}

fn build_transcript_lines(app: &TuiApp) -> Vec<Line<'static>> {
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
    if let Some(live) = &app.live {
        if !live.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("orexis › ", bold(app)),
                Span::raw(live.clone()),
            ]));
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
