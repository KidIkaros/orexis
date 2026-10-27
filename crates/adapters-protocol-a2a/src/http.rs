//! The HTTP shell: A2A over JSON-RPC 2.0 on HTTP.
//!
//! The shell owns the socket, the headers, and the framing; the core owns
//! the protocol shapes. A card is fetched first (the peer tells the client
//! which method naming and which capabilities it speaks), then requests go
//! out as one JSON-RPC message per POST.
//!
//! Streaming, where the peer advertises it, is an SSE response whose
//! frames carry JSON-RPC responses holding a `StreamResponse` — a task, a
//! message, a status update, or an artifact update.

use std::sync::Arc;
use std::time::Duration;

use futures::TryStreamExt;
use orexis_adapters_http::{Transport, TransportError};
use orexis_adapters_jsonrpc::{parse_line, Message, RequestId, SseDecoder};
use serde_json::Value;

use crate::core::{read_message, A2aCore, A2aError, A2aEvent, AgentCard, Outcome};

/// Where a card lives, relative to the peer's base URL.
const CARD_PATH: &str = "/.well-known/agent-card.json";

/// How to reach one remote agent.
#[derive(Debug, Clone, PartialEq)]
pub struct A2aClientConfig {
    /// The name this peer is known by; it namespaces the tool surface.
    pub name: String,
    /// The peer's base URL. The card is fetched from
    /// `<base>/.well-known/agent-card.json`.
    pub base_url: String,
    /// Extra request headers (authorization, and so on).
    pub headers: Vec<(String, String)>,
    /// The agent's own endpoint, when it is known without a card.
    pub endpoint: Option<String>,
}

impl A2aClientConfig {
    pub fn new(name: impl Into<String>, base_url: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            base_url: base_url.into(),
            headers: Vec::new(),
            endpoint: None,
        }
    }

    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = Some(endpoint.into());
        self
    }

    /// The URL the card is fetched from.
    fn card_url(&self) -> String {
        format!("{}{}", self.base_url.trim_end_matches('/'), CARD_PATH)
    }
}

/// An A2A client speaking to one remote agent.
pub struct A2aClient {
    transport: Arc<dyn Transport>,
    config: A2aClientConfig,
    core: Option<A2aCore>,
    card: Option<AgentCard>,
    timeout: Duration,
}

impl A2aClient {
    pub fn new(config: A2aClientConfig, transport: Arc<dyn Transport>, timeout: Duration) -> Self {
        Self {
            transport,
            config,
            core: None,
            card: None,
            timeout,
        }
    }

    /// The peer this client talks to.
    pub fn config(&self) -> &A2aClientConfig {
        &self.config
    }

    /// The card, once it has been fetched.
    pub fn card(&self) -> Option<&AgentCard> {
        self.card.as_ref()
    }

    /// Fetch the agent card and settle the method naming from it.
    pub async fn discover(&mut self) -> Result<AgentCard, A2aError> {
        let response = self
            .get(&self.config.card_url())
            .await
            .map_err(map_transport)?;
        if response.status == 404 {
            return Err(A2aError::NoCard);
        }
        if !(200..300).contains(&response.status) {
            return Err(A2aError::Status(response.status));
        }
        let body = read_body(response).await.map_err(map_transport)?;
        let value: Value = serde_json::from_str(&body)
            .map_err(|error| A2aError::Shape(format!("the agent card is not JSON: {error}")))?;
        let card = crate::core::parse_card(&value)?;
        let endpoint = card.url.clone();
        self.core = Some(A2aCore::new(card.naming(), endpoint));
        self.card = Some(card.clone());
        Ok(card)
    }

    /// Send a message to the agent and read the outcome.
    pub async fn send_message(&mut self, text: &str) -> Result<Outcome, A2aError> {
        let (id, message) = {
            let core = self.core_mut()?;
            core.send_message(text)
        };
        let result = self.request(id, message).await?;
        self.core_mut()?.read_result(&result)
    }

    /// Read a task back.
    pub async fn get_task(&mut self, task_id: &str) -> Result<Outcome, A2aError> {
        let (id, message) = {
            let core = self.core_mut()?;
            core.get_task(task_id)
        };
        let result = self.request(id, message).await?;
        self.core_mut()?.read_result(&result)
    }

    /// Send a message and stream the task updates, where the peer
    /// supports it.
    pub async fn stream_message(&mut self, text: &str) -> Result<Vec<Outcome>, A2aError> {
        let card = self
            .card
            .as_ref()
            .ok_or_else(|| A2aError::Unsupported("the agent card is not known yet".into()))?;
        if !card.streaming {
            return Err(A2aError::Unsupported(
                "the agent does not advertise streaming".into(),
            ));
        }
        let (endpoint, body) = {
            let core = self.core_mut()?;
            let (_, message) = core.stream_message(text);
            (core.endpoint().to_string(), message.to_value().to_string())
        };
        let response = self.post_stream(&endpoint, body).await?;
        let mut outcomes = Vec::new();
        for event in sse_payloads(&response) {
            let Ok(message) = parse_line(&event) else {
                continue;
            };
            match read_message(message) {
                A2aEvent::Result { result, .. } => {
                    if let Ok(outcome) = self.core_mut()?.read_result(&result) {
                        outcomes.push(outcome);
                    }
                }
                A2aEvent::Failed { error, .. } => {
                    return Err(A2aError::Shape(error.message));
                }
                A2aEvent::Other(_) => {}
            }
        }
        Ok(outcomes)
    }

    /// The core, once the card has settled the naming.
    fn core_mut(&mut self) -> Result<&mut A2aCore, A2aError> {
        self.core
            .as_mut()
            .ok_or_else(|| A2aError::Unsupported("the agent card is not known yet".into()))
    }

    /// POST one request and read its result.
    async fn request(&self, id: RequestId, message: Message) -> Result<Value, A2aError> {
        let endpoint = self
            .core
            .as_ref()
            .map(|core| core.endpoint().to_string())
            .ok_or_else(|| A2aError::Unsupported("the agent card is not known yet".into()))?;
        let body = message.to_value().to_string();
        let response = self.post(&endpoint, body).await.map_err(map_transport)?;
        if !(200..300).contains(&response.status) {
            return Err(A2aError::Status(response.status));
        }
        let text = read_body(response).await.map_err(map_transport)?;
        let reply = parse_line(text.trim()).map_err(A2aError::Parse)?;
        match read_message(reply) {
            A2aEvent::Result { id: got, result } => {
                if got != id {
                    return Err(A2aError::Shape(
                        "the answer does not match the request".into(),
                    ));
                }
                Ok(result)
            }
            A2aEvent::Failed { error, .. } => Err(A2aError::Shape(error.message)),
            A2aEvent::Other(_) => Err(A2aError::Shape(
                "the peer answered with something else".into(),
            )),
        }
    }

    fn request_headers(&self, accept: &str) -> Vec<(String, String)> {
        let mut headers = vec![
            ("content-type".to_string(), "application/json".to_string()),
            ("accept".to_string(), accept.to_string()),
        ];
        headers.extend(self.config.headers.iter().cloned());
        headers
    }

    async fn get(&self, url: &str) -> Result<orexis_adapters_http::HttpResponse, TransportError> {
        tokio::time::timeout(
            self.timeout,
            self.transport
                .get_response(url, self.request_headers("application/json")),
        )
        .await
        .map_err(|_| TransportError::Message("the request timed out".into()))?
    }

    async fn post(
        &self,
        url: &str,
        body: String,
    ) -> Result<orexis_adapters_http::HttpResponse, TransportError> {
        tokio::time::timeout(
            self.timeout,
            self.transport
                .post_response(url, self.request_headers("application/json"), body),
        )
        .await
        .map_err(|_| TransportError::Message("the request timed out".into()))?
    }

    async fn post_stream(&self, url: &str, body: String) -> Result<String, A2aError> {
        let headers = self.request_headers("text/event-stream");
        let response = tokio::time::timeout(
            self.timeout,
            self.transport.post_response(url, headers, body),
        )
        .await
        .map_err(|_| A2aError::Transport("the request timed out".into()))?
        .map_err(map_transport)?;
        if !(200..300).contains(&response.status) {
            return Err(A2aError::Status(response.status));
        }
        read_body(response).await.map_err(map_transport)
    }
}

/// The `data:` payloads of an SSE body.
fn sse_payloads(text: &str) -> Vec<String> {
    let mut decoder = SseDecoder::new();
    let mut payloads = Vec::new();
    for event in decoder.push(text).into_iter().chain(decoder.finish()) {
        let payload = event.data.trim();
        if payload.is_empty() {
            continue;
        }
        payloads.push(payload.to_string());
    }
    payloads
}

/// Read a response body as text.
async fn read_body(response: orexis_adapters_http::HttpResponse) -> Result<String, TransportError> {
    let mut body = Vec::new();
    let mut stream = response.body;
    while let Some(chunk) = stream.try_next().await? {
        body.extend_from_slice(&chunk);
    }
    Ok(String::from_utf8_lossy(&body).to_string())
}

fn map_transport(error: TransportError) -> A2aError {
    A2aError::Transport(error.to_string())
}
