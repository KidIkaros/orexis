//! The NDJSON client/server protocol for `orexis serve`.

use orexis_core::SessionEvent;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufRead, AsyncWrite, AsyncWriteExt};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ClientMsg {
    Attach { session: Option<String> },
    Prompt { text: String },
    Approve { id: u64, yes: bool },
    Interrupt,
    Undo,
    ListSessions,
    Detach,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ServerMsg {
    Attached {
        session_id: String,
    },
    Event {
        seq: u64,
        replay: bool,
        event: SessionEvent,
    },
    ApprovalRequest {
        id: u64,
        tool: String,
        args: String,
    },
    Busy {
        session_id: String,
    },
    Queued {
        session_id: String,
        position: usize,
    },
    Sessions {
        sessions: Vec<SessionInfo>,
    },
    Error {
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub status: String,
}

pub async fn write_msg<W: AsyncWrite + Unpin>(
    writer: &mut W,
    message: &ServerMsg,
) -> std::io::Result<()> {
    let mut line = serde_json::to_vec(message)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    line.push(b'\n');
    writer.write_all(&line).await?;
    writer.flush().await
}

pub async fn read_msg<R: AsyncBufRead + Unpin>(
    reader: &mut R,
    line: &mut String,
) -> std::io::Result<Option<ClientMsg>> {
    line.clear();
    let read = tokio::io::AsyncBufReadExt::read_line(reader, line).await?;
    if read == 0 {
        return Ok(None);
    }
    serde_json::from_str(line.trim())
        .map(Some)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
}
