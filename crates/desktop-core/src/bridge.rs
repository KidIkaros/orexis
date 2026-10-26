//! The desktop's bridge to the local daemon.

use std::io;
use std::path::Path;

use orexis_client::DaemonClient;
use orexis_protocol::ServerMsg;

use crate::state::AppState;

pub struct DesktopBridge {
    client: DaemonClient,
    state: AppState,
}

impl DesktopBridge {
    pub fn connect(socket: &Path, session: Option<String>) -> io::Result<Self> {
        let mut client = DaemonClient::connect(socket)?;
        let session_id = client.attach(session)?;
        let mut state = AppState::default();
        state.apply(ServerMsg::Attached { session_id });
        Ok(Self { client, state })
    }

    pub fn state(&self) -> &AppState {
        &self.state
    }

    /// Drain daemon messages, applying each to the state.
    pub fn pump(&mut self) -> io::Result<Vec<ServerMsg>> {
        let mut messages = Vec::new();
        loop {
            match self.client.try_recv() {
                Ok(Some(message)) => {
                    self.state.apply(message.clone());
                    messages.push(message);
                }
                Ok(None) => break,
                Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => {
                    self.state.set_disconnected();
                    break;
                }
                Err(error) => {
                    self.state.set_disconnected();
                    return Err(error);
                }
            }
        }
        Ok(messages)
    }

    pub fn prompt(&mut self, text: &str, skip_verify: bool) -> io::Result<()> {
        self.client.prompt(text, skip_verify)
    }

    pub fn approve(&mut self, id: u64, yes: bool) -> io::Result<()> {
        self.client.approve(id, yes)
    }

    pub fn settings(&mut self) -> io::Result<()> {
        self.client.settings()
    }

    pub fn list_dir(&mut self, path: &str) -> io::Result<()> {
        self.client.list_dir(path)
    }

    pub fn read_file(&mut self, path: &str) -> io::Result<()> {
        self.client.read_file(path)
    }

    pub fn artifacts(&mut self) -> io::Result<()> {
        self.client.artifacts()
    }

    pub fn diff(&mut self) -> io::Result<()> {
        self.client.diff()
    }

    pub fn undo(&mut self) -> io::Result<()> {
        self.client.undo()
    }

    pub fn interrupt(&mut self) -> io::Result<()> {
        self.client.interrupt()
    }

    pub fn list_sessions(&mut self) -> io::Result<()> {
        self.client.list_sessions()
    }

    pub fn detach(&mut self) -> io::Result<()> {
        self.client.detach()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use orexis_core::SessionEvent;
    use orexis_protocol::ClientMsg;
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixListener;
    use std::time::Duration;

    fn write(stream: &mut std::os::unix::net::UnixStream, message: &ServerMsg) {
        let mut line = serde_json::to_string(message).expect("server msg");
        line.push('\n');
        stream.write_all(line.as_bytes()).expect("write");
        stream.flush().expect("flush");
    }

    #[test]
    fn connect_and_pump_applies_events() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("daemon.sock");
        let listener = UnixListener::bind(&path).expect("bind");
        let handle = std::thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept");
            let mut reader = BufReader::new(stream.try_clone().expect("clone"));
            let mut writer = stream;
            let mut line = String::new();
            reader.read_line(&mut line).expect("read");
            let message: ClientMsg = serde_json::from_str(line.trim()).expect("client msg");
            assert!(matches!(message, ClientMsg::Attach { .. }));
            write(
                &mut writer,
                &ServerMsg::Attached {
                    session_id: "s1".to_string(),
                },
            );
            write(
                &mut writer,
                &ServerMsg::Event {
                    seq: 1,
                    replay: false,
                    event: SessionEvent::UserMessage {
                        text: "refactor".to_string(),
                    },
                },
            );
            std::thread::sleep(Duration::from_millis(100));
        });

        let mut bridge = DesktopBridge::connect(&path, None).expect("connect");
        for _ in 0..50 {
            bridge.pump().expect("pump");
            if !bridge.state().transcript.is_empty() {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(bridge.state().transcript.len(), 1);
        assert_eq!(bridge.state().session.as_deref(), Some("s1"));
        handle.join().expect("join");
    }

    #[test]
    fn eof_marks_the_desktop_disconnected() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("daemon.sock");
        let listener = UnixListener::bind(&path).expect("bind");
        let handle = std::thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept");
            let mut reader = BufReader::new(stream.try_clone().expect("clone"));
            let mut writer = stream;
            let mut line = String::new();
            reader.read_line(&mut line).expect("read");
            write(
                &mut writer,
                &ServerMsg::Attached {
                    session_id: "s1".to_string(),
                },
            );
            drop(writer);
        });

        let mut bridge = DesktopBridge::connect(&path, None).expect("connect");
        for _ in 0..50 {
            bridge.pump().expect("pump");
            if bridge.state().connection == crate::Connection::Disconnected {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(bridge.state().connection, crate::Connection::Disconnected);
        handle.join().expect("join");
    }
}
