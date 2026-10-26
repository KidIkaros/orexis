//! A thin, synchronous daemon client for the TUI.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::{Duration, Instant};

use orexis_protocol::{ClientMsg, ServerMsg};

pub struct DaemonClient {
    writer: UnixStream,
    reader: BufReader<UnixStream>,
}

impl DaemonClient {
    pub fn connect(path: &Path) -> std::io::Result<Self> {
        let stream = UnixStream::connect(path)?;
        stream.set_read_timeout(Some(Duration::from_millis(50)))?;
        let reader = BufReader::new(stream.try_clone()?);
        Ok(Self {
            writer: stream,
            reader,
        })
    }

    pub fn send(&mut self, message: &ClientMsg) -> std::io::Result<()> {
        let mut line = serde_json::to_string(message)
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
        line.push('\n');
        self.writer.write_all(line.as_bytes())?;
        self.writer.flush()
    }

    pub fn try_recv(&mut self) -> std::io::Result<Option<ServerMsg>> {
        let mut line = String::new();
        match self.reader.read_line(&mut line) {
            Ok(0) => Ok(None),
            Ok(_) => serde_json::from_str(line.trim())
                .map(Some)
                .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error)),
            Err(error)
                if error.kind() == std::io::ErrorKind::WouldBlock
                    || error.kind() == std::io::ErrorKind::TimedOut =>
            {
                Ok(None)
            }
            Err(error) => Err(error),
        }
    }

    pub fn attach(&mut self, session: Option<String>) -> std::io::Result<String> {
        self.send(&ClientMsg::Attach { session })?;
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if let Some(ServerMsg::Attached { session_id }) = self.try_recv()? {
                return Ok(session_id);
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            "attach timed out",
        ))
    }

    pub fn prompt(&mut self, text: &str, skip_verify: bool) -> std::io::Result<()> {
        self.send(&ClientMsg::Prompt {
            text: text.to_string(),
            skip_verify,
        })
    }

    pub fn approve(&mut self, id: u64, yes: bool) -> std::io::Result<()> {
        self.send(&ClientMsg::Approve { id, yes })
    }

    pub fn interrupt(&mut self) -> std::io::Result<()> {
        self.send(&ClientMsg::Interrupt)
    }

    pub fn undo(&mut self) -> std::io::Result<()> {
        self.send(&ClientMsg::Undo)
    }

    pub fn list_sessions(&mut self) -> std::io::Result<()> {
        self.send(&ClientMsg::ListSessions)
    }

    pub fn detach(&mut self) -> std::io::Result<()> {
        self.send(&ClientMsg::Detach)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use orexis_core::SessionEvent;
    use std::os::unix::net::UnixListener;

    fn write(stream: &mut UnixStream, message: &ServerMsg) {
        let mut line = serde_json::to_string(message).expect("server msg");
        line.push('\n');
        stream.write_all(line.as_bytes()).expect("write");
        stream.flush().expect("flush");
    }

    #[test]
    fn attach_and_receive_an_event() {
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
                    replay: true,
                    event: SessionEvent::Done {
                        summary: "done".to_string(),
                    },
                },
            );
        });

        let mut client = DaemonClient::connect(&path).expect("connect");
        assert_eq!(client.attach(None).expect("attach"), "s1");
        let mut received = None;
        for _ in 0..50 {
            if let Some(message) = client.try_recv().expect("recv") {
                received = Some(message);
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(matches!(received, Some(ServerMsg::Event { .. })));
        handle.join().expect("join");
    }
}
