//! A mock LSP server for tests: initialize, didOpen → one diagnostic.

use serde_json::{json, Value};
use tokio::io::{
    AsyncBufRead, AsyncBufReadExt, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader,
};

#[tokio::main(flavor = "current_thread")]
async fn main() -> std::io::Result<()> {
    let mut stdin = BufReader::new(tokio::io::stdin());
    let mut stdout = tokio::io::stdout();
    loop {
        let Some(message) = read_message(&mut stdin).await? else {
            break;
        };
        match message.get("method").and_then(Value::as_str) {
            Some("initialize") => {
                write_message(
                    &mut stdout,
                    &json!({
                        "jsonrpc": "2.0",
                        "id": message["id"],
                        "result": { "capabilities": {} }
                    }),
                )
                .await?;
            }
            Some("textDocument/didOpen") | Some("textDocument/didChange") => {
                let uri = message["params"]["textDocument"]["uri"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();
                write_message(
                    &mut stdout,
                    &json!({
                        "jsonrpc": "2.0",
                        "method": "textDocument/publishDiagnostics",
                        "params": {
                            "uri": uri,
                            "diagnostics": [{
                                "range": {
                                    "start": { "line": 0, "character": 0 },
                                    "end": { "line": 0, "character": 1 }
                                },
                                "severity": 1,
                                "message": "mock diagnostic"
                            }]
                        }
                    }),
                )
                .await?;
            }
            Some("shutdown") => {
                write_message(
                    &mut stdout,
                    &json!({ "jsonrpc": "2.0", "id": message["id"], "result": null }),
                )
                .await?;
            }
            Some("exit") => break,
            _ => {}
        }
    }
    Ok(())
}

async fn write_message<W: AsyncWrite + Unpin>(
    writer: &mut W,
    value: &Value,
) -> std::io::Result<()> {
    let body = serde_json::to_vec(value)?;
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    writer.write_all(header.as_bytes()).await?;
    writer.write_all(&body).await?;
    writer.flush().await
}

async fn read_message<R: AsyncBufRead + Unpin>(reader: &mut R) -> std::io::Result<Option<Value>> {
    let mut length = None;
    loop {
        let mut line = String::new();
        let read = reader.read_line(&mut line).await?;
        if read == 0 {
            return Ok(None);
        }
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            break;
        }
        if let Some(value) = trimmed.strip_prefix("Content-Length:") {
            length = value.trim().parse::<usize>().ok();
        }
    }
    let length = length.ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, "missing Content-Length")
    })?;
    let mut buffer = vec![0u8; length];
    reader.read_exact(&mut buffer).await?;
    let value = serde_json::from_slice(&buffer)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    Ok(Some(value))
}
