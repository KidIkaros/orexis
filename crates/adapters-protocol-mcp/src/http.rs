//! The streamable HTTP shell: MCP over one HTTP endpoint.
//!
//! The sans-io `McpCore` speaks newline-delimited JSON-RPC; this shell
//! adapts the streamable HTTP transport (MCP spec 2025-06-18) onto it.
//! One JSON-RPC message goes out per POST; the reply comes back either as
//! `application/json` (one message) or as `text/event-stream` (a stream of
//! messages, possibly with server-initiated requests interleaved).
//!
//! Session handling follows the spec: the server may assign an
//! `Mcp-Session-Id` at initialization, and every later request carries it.
//! A 404 on a session-carrying request means the session expired — the
//! shell reports that as an error rather than silently re-initializing, so
//! the caller decides whether to start over.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use futures::TryStreamExt;
use orexis_adapters_http::{HttpResponse, Transport};
use orexis_adapters_jsonrpc::{Message, RequestId, SseDecoder};
use serde_json::Value;

use crate::core::{Event, McpCore};
use crate::error::McpError;
use crate::types::{
    ClientInfo, Initialization, McpPrompt, McpResource, ResourceContent, ToolCallResult,
};

/// The header the server uses to assign a session.
const SESSION_HEADER: &str = "mcp-session-id";
/// The header carrying the negotiated protocol version on later requests.
const PROTOCOL_HEADER: &str = "mcp-protocol-version";

/// How to reach one MCP server over HTTP.
#[derive(Debug, Clone, PartialEq)]
pub struct McpHttpConfig {
    pub name: String,
    pub url: String,
    /// Extra request headers (authorization, and so on).
    pub headers: Vec<(String, String)>,
}

impl McpHttpConfig {
    pub fn new(name: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            url: url.into(),
            headers: Vec::new(),
        }
    }

    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }
}

/// An MCP client speaking streamable HTTP.
pub struct McpHttpClient {
    core: McpCore,
    transport: Arc<dyn Transport>,
    config: McpHttpConfig,
    /// The session the server assigned at initialization, if any.
    session_id: Option<String>,
    /// The protocol version negotiated at initialization.
    protocol_version: Option<String>,
    /// Server-initiated requests already refused, so none is answered twice.
    refused: HashSet<RequestId>,
    timeout: Duration,
}

impl McpHttpClient {
    pub fn new(config: McpHttpConfig, transport: Arc<dyn Transport>, timeout: Duration) -> Self {
        Self {
            core: McpCore::new(ClientInfo {
                name: "orexis".into(),
                version: env!("CARGO_PKG_VERSION").into(),
            }),
            transport,
            config,
            session_id: None,
            protocol_version: None,
            refused: HashSet::new(),
            timeout,
        }
    }

    /// The session id the server assigned, once initialization has run.
    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    /// The negotiated protocol version, once initialization has run.
    pub fn protocol_version(&self) -> Option<&str> {
        self.protocol_version.as_deref()
    }

    /// Run the initialization handshake and keep the session it returns.
    pub async fn connect(&mut self) -> Result<Initialization, McpError> {
        self.core.initialize()?;
        let events = self.exchange().await?;
        for event in &events {
            if let Event::Initialized(initialization) = event {
                return Ok(initialization.clone());
            }
        }
        Err(McpError::Initialization(
            "the server did not return an initialize result".into(),
        ))
    }

    pub async fn list_tools(
        &mut self,
        cursor: Option<String>,
    ) -> Result<(Vec<orexis_core::ToolSpec>, Option<String>), McpError> {
        self.core.list_tools(cursor)?;
        let events = self.exchange().await?;
        for event in events {
            if let Event::ToolsListed { tools, next_cursor } = event {
                return Ok((tools, next_cursor));
            }
        }
        Err(McpError::Transport("no tools/list result".into()))
    }

    pub async fn call_tool(
        &mut self,
        name: &str,
        arguments: Option<Value>,
    ) -> Result<ToolCallResult, McpError> {
        self.core.call_tool(name, arguments)?;
        let events = self.exchange().await?;
        for event in events {
            if let Event::ToolCalled(result) = event {
                return Ok(result);
            }
        }
        Err(McpError::Transport("no tools/call result".into()))
    }

    pub async fn list_resources(
        &mut self,
        cursor: Option<String>,
    ) -> Result<(Vec<McpResource>, Option<String>), McpError> {
        self.core.list_resources(cursor)?;
        let events = self.exchange().await?;
        for event in events {
            if let Event::ResourcesListed {
                resources,
                next_cursor,
            } = event
            {
                return Ok((resources, next_cursor));
            }
        }
        Err(McpError::Transport("no resources/list result".into()))
    }

    pub async fn read_resource(&mut self, uri: &str) -> Result<Vec<ResourceContent>, McpError> {
        self.core.read_resource(uri)?;
        let events = self.exchange().await?;
        for event in events {
            if let Event::ResourceRead { contents } = event {
                return Ok(contents);
            }
        }
        Err(McpError::Transport("no resources/read result".into()))
    }

    pub async fn list_prompts(
        &mut self,
        cursor: Option<String>,
    ) -> Result<(Vec<McpPrompt>, Option<String>), McpError> {
        self.core.list_prompts(cursor)?;
        let events = self.exchange().await?;
        for event in events {
            if let Event::PromptsListed {
                prompts,
                next_cursor,
            } = event
            {
                return Ok((prompts, next_cursor));
            }
        }
        Err(McpError::Transport("no prompts/list result".into()))
    }

    pub async fn get_prompt(
        &mut self,
        name: &str,
        arguments: Option<Value>,
    ) -> Result<Value, McpError> {
        self.core.get_prompt(name, arguments)?;
        let events = self.exchange().await?;
        for event in events {
            if let Event::PromptGot(value) = event {
                return Ok(value);
            }
        }
        Err(McpError::Transport("no prompts/get result".into()))
    }

    pub async fn ping(&mut self) -> Result<(), McpError> {
        self.core.ping()?;
        let events = self.exchange().await?;
        if events.iter().any(|event| matches!(event, Event::Pong)) {
            Ok(())
        } else {
            Err(McpError::Transport("no ping result".into()))
        }
    }

    /// Send everything the core has queued and collect the events the
    /// replies produce. A server-initiated request that arrives inside the
    /// stream is refused once; the refusal is another POST, so the loop
    /// continues until the core has nothing left to send.
    async fn exchange(&mut self) -> Result<Vec<Event>, McpError> {
        let mut events = Vec::new();
        while let Some(message) = self.core.pop_outgoing() {
            let reply = self.send(&message).await?;
            for line in reply {
                let produced = self.core.receive(&line)?;
                for event in &produced {
                    match event {
                        // The negotiated version is learned from the
                        // initialize result, so it is recorded the moment
                        // that result arrives — every later request, the
                        // initialized notification included, carries it.
                        Event::Initialized(initialization) => {
                            self.protocol_version = Some(initialization.protocol_version.clone());
                        }
                        Event::ServerRequest { id, .. } if self.refused.insert(id.clone()) => {
                            self.core.refuse(id.clone());
                        }
                        _ => {}
                    }
                }
                events.extend(produced);
            }
        }
        Ok(events)
    }

    /// POST one JSON-RPC message; return the reply lines it produced.
    async fn send(&mut self, message: &Message) -> Result<Vec<String>, McpError> {
        let body = message.to_value().to_string();
        let response = tokio::time::timeout(
            self.timeout,
            self.transport
                .post_response(&self.config.url, self.request_headers(), body),
        )
        .await
        .map_err(|_| McpError::Timeout)?
        .map_err(|error| McpError::Transport(error.to_string()))?;
        self.absorb_session(&response);
        if response.status == 404 && self.session_id.is_some() {
            return Err(McpError::SessionExpired);
        }
        if !response.is_success() {
            return Err(McpError::Transport(format!(
                "the server answered HTTP {}",
                response.status
            )));
        }
        let content_type = response.content_type();
        let text = read_body(response).await?;
        if content_type == "text/event-stream" {
            return Ok(sse_lines(&text));
        }
        if text.trim().is_empty() {
            return Ok(Vec::new());
        }
        Ok(text.lines().map(str::to_string).collect())
    }

    fn request_headers(&self) -> Vec<(String, String)> {
        let mut headers = vec![
            ("content-type".to_string(), "application/json".to_string()),
            (
                "accept".to_string(),
                "application/json, text/event-stream".to_string(),
            ),
        ];
        if let Some(session) = &self.session_id {
            headers.push((SESSION_HEADER.to_string(), session.clone()));
        }
        if let Some(version) = &self.protocol_version {
            headers.push((PROTOCOL_HEADER.to_string(), version.clone()));
        }
        headers.extend(self.config.headers.iter().cloned());
        headers
    }

    /// Record the session the server assigned at initialization.
    fn absorb_session(&mut self, response: &HttpResponse) {
        if let Some(session) = response.header(SESSION_HEADER) {
            if !session.is_empty() {
                self.session_id = Some(session.to_string());
            }
        }
    }
}

/// Read a response body as text.
async fn read_body(response: HttpResponse) -> Result<String, McpError> {
    let mut body = Vec::new();
    let mut stream = response.body;
    while let Some(chunk) = stream
        .try_next()
        .await
        .map_err(|error| McpError::Transport(error.to_string()))?
    {
        body.extend_from_slice(&chunk);
    }
    Ok(String::from_utf8_lossy(&body).to_string())
}

/// The JSON payloads of an SSE body, one per message.
fn sse_lines(text: &str) -> Vec<String> {
    let mut decoder = SseDecoder::new();
    let mut lines = Vec::new();
    for event in decoder.push(text).into_iter().chain(decoder.finish()) {
        let payload = event.data.trim();
        if payload.is_empty() {
            continue;
        }
        lines.push(payload.to_string());
    }
    lines
}
