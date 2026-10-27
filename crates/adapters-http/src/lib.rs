//! Shared HTTP transport seam for provider adapters.

use std::pin::Pin;
use std::sync::Mutex;

use async_trait::async_trait;
use bytes::Bytes;
use futures::{Stream, TryStreamExt};

#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    #[error("{0}")]
    Message(String),
}

pub type ByteStream = Pin<Box<dyn Stream<Item = Result<Bytes, TransportError>> + Send>>;

/// A response with its status line, headers, and an unread body.
///
/// Protocol shells need more than a body: MCP reads the `Mcp-Session-Id`
/// header and discriminates `application/json` from `text/event-stream`;
/// A2A reads the card's status to tell "no card" from "broken peer".
pub struct HttpResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: ByteStream,
}

impl std::fmt::Debug for HttpResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpResponse")
            .field("status", &self.status)
            .field("headers", &self.headers)
            .finish_non_exhaustive()
    }
}

impl HttpResponse {
    /// A header by name, matched case-insensitively.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    /// The `content-type` header, lowercased and without parameters.
    pub fn content_type(&self) -> String {
        self.header("content-type")
            .unwrap_or_default()
            .split(';')
            .next()
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
    }

    /// True when the status is 2xx.
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }
}

#[async_trait]
pub trait Transport: Send + Sync {
    async fn post_stream(
        &self,
        url: &str,
        headers: Vec<(String, String)>,
        body: String,
    ) -> Result<ByteStream, TransportError>;

    /// A plain GET returning the whole body (tool discovery, agent cards).
    async fn get(
        &self,
        url: &str,
        headers: Vec<(String, String)>,
    ) -> Result<String, TransportError> {
        let mut stream = self.post_stream(url, headers, String::new()).await?;
        let mut body = Vec::new();
        while let Some(chunk) = stream.try_next().await? {
            body.extend_from_slice(&chunk);
        }
        Ok(String::from_utf8_lossy(&body).to_string())
    }

    /// A plain POST returning the whole body.
    async fn post(
        &self,
        url: &str,
        headers: Vec<(String, String)>,
        body: String,
    ) -> Result<String, TransportError> {
        let mut stream = self.post_stream(url, headers, body).await?;
        let mut collected = Vec::new();
        while let Some(chunk) = stream.try_next().await? {
            collected.extend_from_slice(&chunk);
        }
        Ok(String::from_utf8_lossy(&collected).to_string())
    }

    /// A POST that keeps the status line and headers. Protocol shells need
    /// them: MCP carries its session in a header and answers with either
    /// JSON or an event stream.
    async fn post_response(
        &self,
        url: &str,
        headers: Vec<(String, String)>,
        body: String,
    ) -> Result<HttpResponse, TransportError>;

    /// A GET that keeps the status line and headers.
    async fn get_response(
        &self,
        url: &str,
        headers: Vec<(String, String)>,
    ) -> Result<HttpResponse, TransportError>;
}

pub struct ReqwestTransport {
    client: reqwest::Client,
}

impl ReqwestTransport {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    async fn body(response: reqwest::Response) -> Result<String, TransportError> {
        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|error| TransportError::Message(error.to_string()))?;
        if !status.is_success() {
            return Err(TransportError::Message(format!("{status}: {text}")));
        }
        Ok(text)
    }

    /// The status and headers are preserved even for an error status: a
    /// protocol shell decides what a 404 means (a missing card, an expired
    /// session) before it decides whether the body is an error.
    async fn response(response: reqwest::Response) -> Result<HttpResponse, TransportError> {
        let status = response.status().as_u16();
        let headers = response
            .headers()
            .iter()
            .map(|(name, value)| {
                (
                    name.as_str().to_string(),
                    value.to_str().unwrap_or_default().to_string(),
                )
            })
            .collect();
        let stream = response
            .bytes_stream()
            .map_err(|error| TransportError::Message(error.to_string()));
        Ok(HttpResponse {
            status,
            headers,
            body: Box::pin(stream),
        })
    }
}

impl Default for ReqwestTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Transport for ReqwestTransport {
    async fn post_stream(
        &self,
        url: &str,
        headers: Vec<(String, String)>,
        body: String,
    ) -> Result<ByteStream, TransportError> {
        let mut request = self
            .client
            .post(url)
            .header("content-type", "application/json")
            .body(body);
        for (name, value) in headers {
            request = request.header(name, value);
        }
        let response = request
            .send()
            .await
            .map_err(|error| TransportError::Message(error.to_string()))?;
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(TransportError::Message(format!("{status}: {text}")));
        }
        let stream = response
            .bytes_stream()
            .map_err(|error| TransportError::Message(error.to_string()));
        Ok(Box::pin(stream))
    }

    async fn get(
        &self,
        url: &str,
        headers: Vec<(String, String)>,
    ) -> Result<String, TransportError> {
        let mut request = self.client.get(url);
        for (name, value) in headers {
            request = request.header(name, value);
        }
        let response = request
            .send()
            .await
            .map_err(|error| TransportError::Message(error.to_string()))?;
        Self::body(response).await
    }

    async fn post(
        &self,
        url: &str,
        headers: Vec<(String, String)>,
        body: String,
    ) -> Result<String, TransportError> {
        let mut request = self
            .client
            .post(url)
            .header("content-type", "application/json")
            .body(body);
        for (name, value) in headers {
            request = request.header(name, value);
        }
        let response = request
            .send()
            .await
            .map_err(|error| TransportError::Message(error.to_string()))?;
        Self::body(response).await
    }

    async fn post_response(
        &self,
        url: &str,
        headers: Vec<(String, String)>,
        body: String,
    ) -> Result<HttpResponse, TransportError> {
        let mut request = self
            .client
            .post(url)
            .header("content-type", "application/json")
            .body(body);
        for (name, value) in headers {
            request = request.header(name, value);
        }
        let response = request
            .send()
            .await
            .map_err(|error| TransportError::Message(error.to_string()))?;
        Self::response(response).await
    }

    async fn get_response(
        &self,
        url: &str,
        headers: Vec<(String, String)>,
    ) -> Result<HttpResponse, TransportError> {
        let mut request = self.client.get(url);
        for (name, value) in headers {
            request = request.header(name, value);
        }
        let response = request
            .send()
            .await
            .map_err(|error| TransportError::Message(error.to_string()))?;
        Self::response(response).await
    }
}

#[derive(Default)]
pub struct MockTransport {
    pub chunks: Vec<String>,
    pub headers: Mutex<Vec<(String, String)>>,
    /// Scripted bodies consumed one per `get`/`post` call.
    pub script: Mutex<Vec<String>>,
    /// Recorded (method, url, body) for every `get`/`post`.
    pub calls: Mutex<Vec<(String, String, String)>>,
    /// The request headers of every call, in order. A single slot cannot
    /// prove header propagation on an earlier request: every call
    /// overwrites it.
    pub call_headers: Mutex<Vec<Vec<(String, String)>>>,
    /// Scripted full responses, consumed one per `get_response`/`post_response`.
    /// Status and headers travel with the body so a protocol shell sees the
    /// same shape a real peer sends.
    pub responses: Mutex<Vec<MockResponse>>,
}

/// One scripted HTTP response: status, headers, and body.
#[derive(Debug, Clone, Default)]
pub struct MockResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

impl MockResponse {
    pub fn json(body: impl Into<String>) -> Self {
        Self {
            status: 200,
            headers: vec![("content-type".to_string(), "application/json".to_string())],
            body: body.into(),
        }
    }

    pub fn sse(body: impl Into<String>) -> Self {
        Self {
            status: 200,
            headers: vec![("content-type".to_string(), "text/event-stream".to_string())],
            body: body.into(),
        }
    }

    pub fn with_status(mut self, status: u16) -> Self {
        self.status = status;
        self
    }

    pub fn with_header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_string(), value.to_string()));
        self
    }
}

impl MockTransport {
    pub fn new(chunks: Vec<String>) -> Self {
        Self {
            chunks,
            headers: Mutex::new(Vec::new()),
            script: Mutex::new(Vec::new()),
            calls: Mutex::new(Vec::new()),
            call_headers: Mutex::new(Vec::new()),
            responses: Mutex::new(Vec::new()),
        }
    }

    /// Queue scripted responses for the response-aware calls.
    pub fn with_responses(self, responses: Vec<MockResponse>) -> Self {
        self.responses.lock().expect("responses").extend(responses);
        self
    }
}

#[async_trait]
impl Transport for MockTransport {
    async fn post_stream(
        &self,
        _url: &str,
        headers: Vec<(String, String)>,
        _body: String,
    ) -> Result<ByteStream, TransportError> {
        self.record_headers(headers);
        let chunks = self.chunks.clone();
        Ok(Box::pin(futures::stream::iter(
            chunks.into_iter().map(|chunk| Ok(Bytes::from(chunk))),
        )))
    }

    async fn get(
        &self,
        url: &str,
        headers: Vec<(String, String)>,
    ) -> Result<String, TransportError> {
        self.record_call("GET", url, String::new(), headers);
        self.next_body()
    }

    async fn post(
        &self,
        url: &str,
        headers: Vec<(String, String)>,
        body: String,
    ) -> Result<String, TransportError> {
        self.record_call("POST", url, body, headers);
        self.next_body()
    }

    async fn post_response(
        &self,
        url: &str,
        headers: Vec<(String, String)>,
        body: String,
    ) -> Result<HttpResponse, TransportError> {
        self.record_call("POST", url, body, headers);
        Ok(self.next_response().await?)
    }

    async fn get_response(
        &self,
        url: &str,
        headers: Vec<(String, String)>,
    ) -> Result<HttpResponse, TransportError> {
        self.record_call("GET", url, String::new(), headers);
        Ok(self.next_response().await?)
    }
}

impl MockTransport {
    fn record(&self, method: &str, url: &str, body: String) {
        if let Ok(mut calls) = self.calls.lock() {
            calls.push((method.to_string(), url.to_string(), body));
        }
    }

    /// Record the headers of one call, both as the latest headers and as
    /// that call's own entry.
    fn record_headers(&self, headers: Vec<(String, String)>) {
        if let Ok(mut recorded) = self.headers.lock() {
            *recorded = headers.clone();
        }
        if let Ok(mut history) = self.call_headers.lock() {
            history.push(headers);
        }
    }

    /// Record a call and its headers together.
    fn record_call(&self, method: &str, url: &str, body: String, headers: Vec<(String, String)>) {
        self.record_headers(headers);
        self.record(method, url, body);
    }

    fn next_body(&self) -> Result<String, TransportError> {
        let mut script = self
            .script
            .lock()
            .map_err(|error| TransportError::Message(error.to_string()))?;
        if script.is_empty() {
            return Err(TransportError::Message("no scripted response".into()));
        }
        let body = script.remove(0);
        Ok(body)
    }

    /// The next scripted response; an unqueued call falls back to the
    /// plain body script so a fixture may mix both styles.
    async fn next_response(&self) -> Result<HttpResponse, TransportError> {
        let queued = {
            let mut responses = self
                .responses
                .lock()
                .map_err(|error| TransportError::Message(error.to_string()))?;
            if responses.is_empty() {
                None
            } else {
                Some(responses.remove(0))
            }
        };
        let response = match queued {
            Some(response) => response,
            None => MockResponse {
                status: 200,
                headers: vec![("content-type".to_string(), "application/json".to_string())],
                body: self.next_body()?,
            },
        };
        let body = response.body.clone();
        Ok(HttpResponse {
            status: response.status,
            headers: response.headers,
            body: Box::pin(futures::stream::iter(vec![Ok(Bytes::from(body))])),
        })
    }
}

pub struct FailingTransport {
    pub message: String,
}

#[async_trait]
impl Transport for FailingTransport {
    async fn post_stream(
        &self,
        _url: &str,
        _headers: Vec<(String, String)>,
        _body: String,
    ) -> Result<ByteStream, TransportError> {
        Err(TransportError::Message(self.message.clone()))
    }

    async fn post_response(
        &self,
        _url: &str,
        _headers: Vec<(String, String)>,
        _body: String,
    ) -> Result<HttpResponse, TransportError> {
        Err(TransportError::Message(self.message.clone()))
    }

    async fn get_response(
        &self,
        _url: &str,
        _headers: Vec<(String, String)>,
    ) -> Result<HttpResponse, TransportError> {
        Err(TransportError::Message(self.message.clone()))
    }
}
