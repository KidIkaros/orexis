//! Key handling.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::TuiApp;
use crate::state::{Entry, Focus, Overlay, TuiAction, TuiState};

impl TuiApp {
    pub fn handle_key(&mut self, key: KeyEvent) -> TuiAction {
        match self.state {
            TuiState::Approval => match key.code {
                KeyCode::Char('y') => TuiAction::Approve,
                KeyCode::Char('n') => TuiAction::Deny,
                KeyCode::Char('d') => TuiAction::Diff,
                KeyCode::Char('a') => TuiAction::AlwaysAllow,
                KeyCode::Esc => {
                    self.state = TuiState::Streaming;
                    TuiAction::Cancel
                }
                _ => TuiAction::None,
            },
            TuiState::PlanGate => match key.code {
                KeyCode::Char('a') => TuiAction::PlanApprove,
                KeyCode::Char('e') => TuiAction::PlanEdit,
                KeyCode::Char('c') => {
                    self.state = TuiState::Streaming;
                    TuiAction::PlanCancel
                }
                _ => TuiAction::None,
            },
            _ => self.handle_normal(key),
        }
    }

    fn handle_normal(&mut self, key: KeyEvent) -> TuiAction {
        if let Some(overlay) = self.overlay {
            match key.code {
                KeyCode::Esc => {
                    self.overlay = None;
                    self.focus = Focus::Composer;
                }
                KeyCode::Backspace => match overlay {
                    Overlay::Palette => {
                        self.palette_query.pop();
                    }
                    Overlay::Search => {
                        self.search_query.pop();
                    }
                    _ => {}
                },
                KeyCode::Char(character) => match overlay {
                    Overlay::Palette => {
                        self.palette_query.push(character);
                        self.palette_selected = 0;
                    }
                    Overlay::Search => self.search_query.push(character),
                    _ => {}
                },
                KeyCode::Down if overlay == Overlay::Palette => {
                    self.palette_selected = self.palette_selected.saturating_add(1);
                }
                KeyCode::Up if overlay == Overlay::Palette => {
                    self.palette_selected = self.palette_selected.saturating_sub(1);
                }
                KeyCode::Enter if overlay == Overlay::Palette => {
                    let commands = self.palette_commands();
                    if !commands.is_empty() {
                        let index = self.palette_selected.min(commands.len() - 1);
                        let action = commands[index].1.clone();
                        self.overlay = None;
                        self.focus = Focus::Composer;
                        return action;
                    }
                }
                _ => {}
            }
            return TuiAction::None;
        }

        match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                TuiAction::Interrupt
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                TuiAction::Detach
            }
            KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => TuiAction::Fleet,
            KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.overlay = Some(Overlay::Palette);
                self.focus = Focus::Overlay;
                TuiAction::None
            }
            KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.overlay = Some(Overlay::Search);
                self.focus = Focus::Overlay;
                TuiAction::None
            }
            KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => TuiAction::Board,
            KeyCode::Char('?') => {
                self.overlay = Some(Overlay::Help);
                self.focus = Focus::Overlay;
                TuiAction::None
            }
            KeyCode::Tab => {
                self.focus = match self.focus {
                    Focus::Composer => Focus::Transcript,
                    _ => Focus::Composer,
                };
                TuiAction::None
            }
            KeyCode::Esc => TuiAction::Cancel,
            _ => match self.focus {
                Focus::Composer => self.handle_composer(key),
                Focus::Transcript => self.handle_transcript(key),
                Focus::Overlay => TuiAction::None,
            },
        }
    }

    fn handle_composer(&mut self, key: KeyEvent) -> TuiAction {
        match key.code {
            KeyCode::Enter if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.composer.push('\n');
                TuiAction::None
            }
            KeyCode::Enter => {
                let text = std::mem::take(&mut self.composer);
                TuiAction::Send(text)
            }
            KeyCode::Backspace => {
                self.composer.pop();
                TuiAction::None
            }
            KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                TuiAction::EditExternal
            }
            KeyCode::Char(c) => {
                self.composer.push(c);
                TuiAction::None
            }
            _ => TuiAction::None,
        }
    }

    fn handle_transcript(&mut self, key: KeyEvent) -> TuiAction {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.scroll = self.scroll.saturating_add(1);
                TuiAction::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.scroll = self.scroll.saturating_sub(1);
                TuiAction::None
            }
            KeyCode::Char('g') => {
                self.scroll = 0;
                TuiAction::None
            }
            KeyCode::Char('G') => {
                self.scroll = usize::MAX;
                TuiAction::None
            }
            KeyCode::Enter => {
                if let Some(Entry::Tool(row)) = self.entries.get_mut(self.selected) {
                    row.expanded = !row.expanded;
                    if row.expanded && row.detail.lines().count() > 8 {
                        self.bottom_sheet = Some(row.detail.clone());
                    }
                }
                self.revision += 1;
                TuiAction::None
            }
            KeyCode::Char('o') => {
                if let Some(Entry::Tool(row)) = self.entries.get(self.selected) {
                    return TuiAction::OpenFile(row.target.clone());
                }
                TuiAction::None
            }
            KeyCode::Char('y') => TuiAction::Copy,
            KeyCode::Char('q') => TuiAction::Quit,
            _ => TuiAction::None,
        }
    }

    pub fn keyhints(&self) -> &'static str {
        match self.state {
            TuiState::Approval => "y approve · n deny · d diff · a always",
            TuiState::PlanGate => "a approve · e edit · c cancel",
            _ if self.overlay == Some(Overlay::Palette) => "Enter run · Esc close · ↑/↓ select",
            _ if self.overlay == Some(Overlay::Search) => "type to filter · Esc close",
            _ if self.overlay.is_some() => "Esc close · j/k navigate · Enter select",
            _ if self.focus == Focus::Transcript => {
                "j/k scroll · Enter expand · o open · y copy · Tab composer"
            }
            _ => "Enter send · ⇧Enter newline · Esc cancel · ^P palette · ^D detach · ? help",
        }
    }
}
