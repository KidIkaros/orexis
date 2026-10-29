//! The crossterm event loop and daemon client glue.

use std::io::{self, Write};
use std::path::PathBuf;
use std::time::Duration;

use crossterm::event::{self, DisableBracketedPaste, EnableBracketedPaste, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::DaemonClient;
use crate::{TuiAction, TuiApp, TuiState};

pub fn run(socket: PathBuf, session: Option<String>) -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableBracketedPaste)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let result = run_loop(&mut terminal, socket, session);
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableBracketedPaste
    )?;
    terminal.show_cursor()?;
    result
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    socket: PathBuf,
    session: Option<String>,
) -> io::Result<()> {
    let mut client = DaemonClient::connect(&socket)?;
    let mut app = TuiApp::new();
    app.session = client.attach(session)?;

    loop {
        terminal.draw(|frame| app.render(frame))?;

        if event::poll(Duration::from_millis(30))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    let action = app.handle_key(key);
                    match action {
                        TuiAction::Send(text) => client.prompt(&text, false)?,
                        TuiAction::Approve => {
                            if let Some(id) = app.pending_approval.take() {
                                client.approve(id, true)?;
                            }
                        }
                        TuiAction::Deny => {
                            if let Some(id) = app.pending_approval.take() {
                                client.approve(id, false)?;
                            }
                        }
                        TuiAction::AlwaysAllow => {
                            if let Some(id) = app.pending_approval.take() {
                                client.approve_always(id)?;
                            }
                        }
                        TuiAction::Diff => client.diff()?,
                        TuiAction::Interrupt | TuiAction::Cancel => client.interrupt()?,
                        TuiAction::Undo => client.undo()?,
                        TuiAction::SelfView => client.self_view()?,
                        TuiAction::Evolution => client.evolution()?,
                        TuiAction::Board => client.board()?,
                        TuiAction::Fleet => client.fleet()?,
                        TuiAction::Artifacts => client.artifacts()?,
                        TuiAction::Detach | TuiAction::Quit => {
                            let _ = client.detach();
                            break;
                        }
                        TuiAction::EditExternal => edit_external(terminal, &mut app)?,
                        _ => {}
                    }
                }
                Event::Paste(text) => app.composer.push_str(&text),
                _ => {}
            }
        }

        loop {
            match client.try_recv() {
                Ok(Some(message)) => app.apply(message),
                Ok(None) => break,
                Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => {
                    app.set_state(TuiState::Disconnected);
                    terminal.draw(|frame| app.render(frame))?;
                    let _ = event::read();
                    return Ok(());
                }
                Err(error) => return Err(error),
            }
        }
    }
    Ok(())
}

fn edit_external(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut TuiApp,
) -> io::Result<()> {
    let mut file = tempfile::NamedTempFile::new()?;
    write!(file, "{}", app.composer)?;
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableBracketedPaste
    )?;
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
    let status = std::process::Command::new(editor).arg(file.path()).status();
    execute!(
        terminal.backend_mut(),
        EnterAlternateScreen,
        EnableBracketedPaste
    )?;
    enable_raw_mode()?;
    if status.map(|status| status.success()).unwrap_or(false) {
        app.composer = std::fs::read_to_string(file.path())?;
    }
    Ok(())
}
