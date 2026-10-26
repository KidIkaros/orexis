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
}

#[derive(Default)]
pub struct MockTransport {
    pub chunks: Vec<String>,
    pub headers: Mutex<Vec<(String, String)>>,
}

impl MockTransport {
    pub fn new(chunks: Vec<String>) -> Self {
        Self {
            chunks,
            headers: Mutex::new(Vec::new()),
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
