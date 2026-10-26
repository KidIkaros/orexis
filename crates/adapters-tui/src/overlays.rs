//! Overlays and the bottom sheet.

use ratatui::{
    layout::Rect,
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::render::transcript_lines;
use crate::{Overlay, TuiApp};

pub(crate) fn draw_overlay(frame: &mut Frame, app: &TuiApp, overlay: Overlay, area: Rect) {
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
        Paragraph::new(overlay_lines(app, overlay))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(overlay.title()),
            )
            .wrap(Wrap { trim: false }),
        rect,
    );
}

fn overlay_lines(app: &TuiApp, overlay: Overlay) -> Vec<Line<'static>> {
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
        Overlay::Evolution => match &app.evolution_text {
            Some(text) => text
                .lines()
                .map(|line| Line::from(line.to_string()))
                .collect(),
            None => vec![Line::from("no evolution ledger")],
        },
        Overlay::SelfState => match &app.self_text {
            Some(text) => text
                .lines()
                .map(|line| Line::from(line.to_string()))
                .collect(),
            None => vec![Line::from("no entity state yet")],
        },
        Overlay::Palette => {
            let commands = app.palette_commands();
            if commands.is_empty() {
                return vec![Line::from("no matching commands")];
            }
            commands
                .iter()
                .enumerate()
                .map(|(index, (label, _))| {
                    let marker = if index == app.palette_selected {
                        ">"
                    } else {
                        " "
                    };
                    Line::from(format!("{marker} {label}"))
                })
                .collect()
        }
        Overlay::Search => {
            let query = app.search_query.to_lowercase();
            let lines = transcript_lines(app);
            let matches: Vec<Line<'static>> = lines
                .into_iter()
                .filter(|line| {
                    let text = line
                        .spans
                        .iter()
                        .map(|span| span.content.as_ref())
                        .collect::<String>()
                        .to_lowercase();
                    !query.is_empty() && text.contains(&query)
                })
                .collect();
            if matches.is_empty() {
                vec![Line::from("no matches")]
            } else {
                matches
            }
        }
        Overlay::Help => vec![
            Line::from("Enter send · ⇧Enter newline · Esc cancel"),
            Line::from("Ctrl-C interrupt · Ctrl-D detach · Ctrl-F fleet"),
            Line::from("Ctrl-B board · ? help · Tab focus"),
            Line::from("j/k scroll · Enter expand · o open · y copy"),
            Line::from("approval: y/n/d/a · plan: a/e/c"),
        ],
    }
}

pub(crate) fn draw_bottom_sheet(frame: &mut Frame, sheet: &str, area: Rect) {
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
