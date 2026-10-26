//! A thin, synchronous daemon client for the TUI.

use std::io::{BufReader, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::{Duration, Instant};

use orexis_protocol::{ClientMsg, ServerMsg};

pub struct DaemonClient {
    writer: UnixStream,
    reader: BufReader<UnixStream>,
    pending: Vec<u8>,
}

impl DaemonClient {
    pub fn connect(path: &Path) -> std::io::Result<Self> {
        let stream = UnixStream::connect(path)?;
        stream.set_read_timeout(Some(Duration::from_millis(50)))?;
        let reader = BufReader::new(stream.try_clone()?);
        Ok(Self {
            writer: stream,
            reader,
            pending: Vec::new(),
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
        loop {
            if let Some(index) = self.pending.iter().position(|&byte| byte == b'\n') {
                let line: Vec<u8> = self.pending.drain(..=index).collect();
                let text = std::str::from_utf8(&line)
                    .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
                let trimmed = text.trim();
                if trimmed.is_empty() {
                    continue;
                }
                return serde_json::from_str(trimmed)
                    .map(Some)
                    .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error));
            }
            let mut chunk = [0u8; 4096];
            match self.reader.read(&mut chunk) {
                Ok(0) => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "daemon disconnected",
                    ))
                }
                Ok(read) => self.pending.extend_from_slice(&chunk[..read]),
                Err(error)
                    if error.kind() == std::io::ErrorKind::WouldBlock
                        || error.kind() == std::io::ErrorKind::TimedOut =>
                {
                    return Ok(None)
                }
                Err(error) => return Err(error),
            }
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
    use std::io::BufRead;
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

    #[test]
    fn a_split_message_is_reassembled() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("daemon.sock");
        let listener = UnixListener::bind(&path).expect("bind");
        let handle = std::thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept");
            let mut reader = BufReader::new(stream.try_clone().expect("clone"));
            let mut writer = stream;
            let mut line = String::new();
            reader.read_line(&mut line).expect("read");
            let mut message = serde_json::to_string(&ServerMsg::Event {
                seq: 1,
                replay: true,
                event: SessionEvent::Done {
                    summary: "done".to_string(),
                },
            })
            .expect("msg");
            message.push('\n');
            let split = message.len() / 2;
            writer
                .write_all(&message.as_bytes()[..split])
                .expect("write");
            writer.flush().expect("flush");
            std::thread::sleep(Duration::from_millis(100));
            writer
                .write_all(&message.as_bytes()[split..])
                .expect("write");
            writer.flush().expect("flush");
        });

        let mut client = DaemonClient::connect(&path).expect("connect");
        client
            .send(&ClientMsg::Attach { session: None })
            .expect("attach");
        let mut received = None;
        for _ in 0..50 {
            if let Some(message) = client.try_recv().expect("recv") {
                received = Some(message);
                break;
            }
        }
        assert!(matches!(received, Some(ServerMsg::Event { .. })));
        handle.join().expect("join");
    }

    #[test]
    fn eof_is_an_error() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("daemon.sock");
        let listener = UnixListener::bind(&path).expect("bind");
        let handle = std::thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept");
            drop(stream);
        });
        let mut client = DaemonClient::connect(&path).expect("connect");
        let error = client.try_recv().expect_err("eof");
        assert_eq!(error.kind(), std::io::ErrorKind::UnexpectedEof);
        handle.join().expect("join");
    }
}
