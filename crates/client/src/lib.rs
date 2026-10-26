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
        self.send(&ClientMsg::Approve {
            id,
            yes,
            always: false,
        })
    }

    pub fn interrupt(&mut self) -> std::io::Result<()> {
        self.send(&ClientMsg::Interrupt)
    }

    pub fn diff(&mut self) -> std::io::Result<()> {
        self.send(&ClientMsg::Diff)
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
