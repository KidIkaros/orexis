//! SSE framing: server-sent events decoded into payloads.
//!
//! MCP and A2A both stream JSON-RPC messages over `text/event-stream`.
//! The decoder is sans-io and chunk-agnostic: feed it decoded text, take
//! out complete events. An event is dispatched on a blank line; `data`
//! lines accumulate with `\n` between them, per the SSE standard.

/// One dispatched server-sent event.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SseEvent {
    /// The `event:` field, when the server named the event.
    pub event: Option<String>,
    /// The `data:` field(s), joined with `\n`.
    pub data: String,
    /// The `id:` field, when present — the cursor for resuming a stream.
    pub id: Option<String>,
}

/// Accumulates SSE text and yields complete events.
#[derive(Debug, Default)]
pub struct SseDecoder {
    buffer: String,
    data: Vec<String>,
    event: Option<String>,
    id: Option<String>,
}

impl SseDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed a chunk of the stream; returns every event completed by it.
    pub fn push(&mut self, chunk: &str) -> Vec<SseEvent> {
        self.buffer.push_str(chunk);
        let mut events = Vec::new();
        while let Some(index) = self.buffer.find('\n') {
            let line: String = self.buffer.drain(..=index).collect();
            let line = line.trim_end_matches('\n').trim_end_matches('\r');
            if line.is_empty() {
                if let Some(event) = self.dispatch() {
                    events.push(event);
                }
                continue;
            }
            if line.starts_with(':') {
                continue;
            }
            let (field, value) = match line.split_once(':') {
                Some((field, value)) => (field, value),
                None => (line, ""),
            };
            let value = value.strip_prefix(' ').unwrap_or(value);
            match field {
                "data" => self.data.push(value.to_string()),
                "event" => self.event = Some(value.to_string()),
                "id" => self.id = Some(value.to_string()),
                _ => {}
            }
        }
        events
    }

    /// Flush the tail: a stream that ends without a final blank line
    /// still carries its last event.
    pub fn finish(&mut self) -> Vec<SseEvent> {
        let mut events = Vec::new();
        if !self.buffer.trim().is_empty() {
            let line = std::mem::take(&mut self.buffer);
            let line = line.trim_end_matches('\n').trim_end_matches('\r');
            if line.is_empty() {
                if let Some(event) = self.dispatch() {
                    events.push(event);
                }
            } else if !line.starts_with(':') {
                let (field, value) = match line.split_once(':') {
                    Some((field, value)) => (field, value),
                    None => (line, ""),
                };
                let value = value.strip_prefix(' ').unwrap_or(value);
                match field {
                    "data" => self.data.push(value.to_string()),
                    "event" => self.event = Some(value.to_string()),
                    "id" => self.id = Some(value.to_string()),
                    _ => {}
                }
            }
        }
        if let Some(event) = self.dispatch() {
            events.push(event);
        }
        events
    }

    fn dispatch(&mut self) -> Option<SseEvent> {
        if self.data.is_empty() && self.event.is_none() {
            return None;
        }
        let event = SseEvent {
            event: self.event.take(),
            data: self.data.join("\n"),
            id: self.id.take(),
        };
        self.data.clear();
        Some(event)
    }
}

#[cfg(test)]
mod tests;
