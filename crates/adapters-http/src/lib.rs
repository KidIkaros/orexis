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
}

#[derive(Default)]
pub struct MockTransport {
    pub chunks: Vec<String>,
    pub headers: Mutex<Vec<(String, String)>>,
    /// Scripted bodies consumed one per `get`/`post` call.
    pub script: Mutex<Vec<String>>,
    /// Recorded (method, url, body) for every `get`/`post`.
    pub calls: Mutex<Vec<(String, String, String)>>,
}

impl MockTransport {
    pub fn new(chunks: Vec<String>) -> Self {
        Self {
            chunks,
            headers: Mutex::new(Vec::new()),
            script: Mutex::new(Vec::new()),
            calls: Mutex::new(Vec::new()),
        }
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
        if let Ok(mut recorded) = self.headers.lock() {
            *recorded = headers;
        }
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
        if let Ok(mut recorded) = self.headers.lock() {
            *recorded = headers;
        }
        self.record("GET", url, String::new());
        self.next_body()
    }

    async fn post(
        &self,
        url: &str,
        headers: Vec<(String, String)>,
        body: String,
    ) -> Result<String, TransportError> {
        if let Ok(mut recorded) = self.headers.lock() {
            *recorded = headers;
        }
        self.record("POST", url, body);
        self.next_body()
    }
}

impl MockTransport {
    fn record(&self, method: &str, url: &str, body: String) {
        if let Ok(mut calls) = self.calls.lock() {
            calls.push((method.to_string(), url.to_string(), body));
        }
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
}
