//! NDJSON helpers over the shared protocol types.

pub use orexis_protocol::{Artifact, ClientMsg, FileEntry, ServerMsg, SessionInfo, SettingsInfo};

use tokio::io::{AsyncBufRead, AsyncWrite, AsyncWriteExt};

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
