//! ndjson framing: one JSON-RPC message per line, blank lines skipped.

use serde_json::Value;

use super::message::{parse_value, Message, ParseError};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FramingError {
    #[error("malformed JSON-RPC on line {line}: {source}")]
    Malformed { line: usize, source: ParseError },
}

pub fn encode_line(message: &Message) -> String {
    let mut line = message.to_value().to_string();
    line.push('\n');
    line
}

pub fn decode_str(buffer: &str) -> Result<Vec<Message>, FramingError> {
    let mut messages = Vec::new();
    for (index, line) in buffer.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let value: Value = serde_json::from_str(line).map_err(|error| FramingError::Malformed {
            line: index + 1,
            source: super::message::ParseError::Json(error.to_string()),
        })?;
        let message = parse_value(&value).map_err(|source| FramingError::Malformed {
            line: index + 1,
            source,
        })?;
        messages.push(message);
    }
    Ok(messages)
}

#[cfg(test)]
mod tests;
