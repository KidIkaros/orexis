//! OpenAI-compatible provider adapter: request building and SSE parsing.

use std::collections::BTreeMap;

use async_trait::async_trait;
use futures::StreamExt;
use orexis_adapters_http::{ByteStream, Transport};
use orexis_adapters_provider_catalog::ProviderProfile;
use orexis_core::{Action, Message, Role, ToolSpec};
use orexis_ports::Provider;
use serde_json::{json, Value};

pub struct OpenAiCompatibleProvider<T: Transport> {
    profile: ProviderProfile,
    model: String,
    api_key: Option<String>,
    transport: T,
}

impl<T: Transport> OpenAiCompatibleProvider<T> {
    pub fn new(
        profile: ProviderProfile,
        model: impl Into<String>,
        api_key: Option<String>,
        transport: T,
    ) -> Self {
        Self {
            profile,
            model: model.into(),
            api_key,
            transport,
        }
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    pub fn request_body(&self, messages: &[Message], tools: &[ToolSpec]) -> Value {
        let messages: Vec<Value> = messages
            .iter()
            .map(|message| match message.role {
                Role::System => json!({ "role": "system", "content": message.text }),
                Role::User => json!({ "role": "user", "content": message.text }),
                Role::Assistant => json!({ "role": "assistant", "content": message.text }),
                Role::Tool => json!({
                    "role": "user",
                    "content": format!("[tool result] {}", message.text),
                }),
            })
            .collect();

        let mut body = json!({
            "model": self.model,
            "messages": messages,
            "stream": true,
        });

        if !tools.is_empty() {
            body["tools"] = json!(tools
                .iter()
                .map(|tool| json!({
                    "type": "function",
                    "function": {
                        "name": tool.name,
                        "description": tool.description,
                        "parameters": tool.schema,
                    }
                }))
                .collect::<Vec<_>>());
        }

        body
    }
}

#[async_trait]
impl<T: Transport + 'static> Provider for OpenAiCompatibleProvider<T> {
    async fn complete(
        &self,
        messages: &[Message],
        tools: &[ToolSpec],
        _step: usize,
    ) -> Vec<Action> {
        let url = format!(
            "{}/chat/completions",
            self.profile.base_url.trim_end_matches('/')
        );
        let mut headers = vec![("content-type".to_string(), "application/json".to_string())];
        if let Some(key) = &self.api_key {
            headers.push((
                self.profile.auth.header.clone(),
                format!("{}{}", self.profile.auth.prefix, key),
            ));
        }

        match self
            .transport
            .post_stream(
                &url,
                headers,
                self.request_body(messages, tools).to_string(),
            )
            .await
        {
            Ok(stream) => parse_stream(stream).await,
            Err(error) => vec![
                Action::Say {
                    text: format!("provider error: {error}"),
                },
                Action::Done {
                    summary: "provider error".to_string(),
                },
            ],
        }
    }
}

#[derive(Default)]
struct ToolCallAccum {
    name: String,
    args: String,
}

async fn parse_stream(mut stream: ByteStream) -> Vec<Action> {
    let mut buffer = String::new();
    let mut content = String::new();
    let mut tool_calls: BTreeMap<usize, ToolCallAccum> = BTreeMap::new();
    let mut finish_reason: Option<String> = None;

    while let Some(chunk) = stream.next().await {
        let Ok(bytes) = chunk else {
            break;
        };
        buffer.push_str(&String::from_utf8_lossy(&bytes));

        while let Some(index) = buffer.find("\n\n") {
            let event = buffer[..index].to_string();
            buffer.drain(..index + 2);
            for line in event.lines() {
                let Some(data) = line.strip_prefix("data: ") else {
                    continue;
                };
                if data.trim() == "[DONE]" {
                    continue;
                }
                let Ok(value) = serde_json::from_str::<Value>(data) else {
                    continue;
                };
                let choice = &value["choices"][0];
                if let Some(text) = choice["delta"]["content"].as_str() {
                    content.push_str(text);
                }
                if let Some(calls) = choice["delta"]["tool_calls"].as_array() {
                    for call in calls {
                        let index = call["index"].as_u64().unwrap_or(0) as usize;
                        let entry = tool_calls.entry(index).or_default();
                        if let Some(name) = call["function"]["name"].as_str() {
                            entry.name = name.to_string();
                        }
                        if let Some(args) = call["function"]["arguments"].as_str() {
                            entry.args.push_str(args);
                        }
                    }
                }
                if let Some(reason) = choice["finish_reason"].as_str() {
                    finish_reason = Some(reason.to_string());
                }
            }
        }
    }

    let mut actions = Vec::new();
    if !tool_calls.is_empty() {
        for (_, call) in tool_calls {
            actions.push(Action::CallTool {
                name: call.name,
                args: call.args,
            });
        }
    } else if !content.is_empty() {
        actions.push(Action::Say {
            text: content.clone(),
        });
        if finish_reason.as_deref() == Some("stop") {
            actions.push(Action::Done { summary: content });
        }
    } else if finish_reason.is_some() {
        actions.push(Action::Done {
            summary: "done".to_string(),
        });
    }
    actions
}

#[cfg(test)]
mod tests {
    use super::*;
    use orexis_adapters_http::MockTransport;
    use orexis_core::Role;

    fn profile() -> ProviderProfile {
        serde_yaml::from_str(include_str!(
            "../../adapters-provider-catalog/providers/openai.yaml"
        ))
        .expect("profile")
    }

    fn provider(chunks: Vec<&str>) -> OpenAiCompatibleProvider<MockTransport> {
        OpenAiCompatibleProvider::new(
            profile(),
            "gpt-5",
            Some("test-key".to_string()),
            MockTransport::new(chunks.into_iter().map(str::to_string).collect()),
        )
    }

    fn messages() -> Vec<Message> {
        vec![Message {
            role: Role::User,
            text: "hi".to_string(),
        }]
    }

    #[tokio::test]
    async fn content_stream_becomes_say_and_done() {
        let provider = provider(vec![
            "data: {\"choices\":[{\"delta\":{\"content\":\"hel\"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"lo\"},\"finish_reason\":\"stop\"}]}\n\n",
            "data: [DONE]\n\n",
        ]);
        let actions = provider.complete(&messages(), &[], 0).await;
        assert_eq!(
            actions,
            vec![
                Action::Say {
                    text: "hello".to_string()
                },
                Action::Done {
                    summary: "hello".to_string()
                }
            ]
        );
    }

    #[tokio::test]
    async fn tool_call_stream_becomes_call_tool() {
        let provider = provider(vec![
            "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"name\":\"read\",\"arguments\":\"{\\\"path\\\":\"}}]}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"arguments\":\"\\\"x\\\"}\"}}]}}]}\n\n",
            "data: [DONE]\n\n",
        ]);
        let actions = provider.complete(&messages(), &[], 0).await;
        assert_eq!(
            actions,
            vec![Action::CallTool {
                name: "read".to_string(),
                args: "{\"path\":\"x\"}".to_string()
            }]
        );
    }

    #[tokio::test]
    async fn auth_header_uses_the_profile() {
        let provider = provider(vec!["data: [DONE]\n\n"]);
        let _ = provider.complete(&messages(), &[], 0).await;
        let headers = provider.transport().headers.lock().unwrap();
        assert!(headers
            .iter()
            .any(|(name, value)| name == "authorization" && value == "Bearer test-key"));
    }

    #[test]
    fn body_includes_tools() {
        let provider = provider(vec![]);
        let tools = vec![ToolSpec {
            name: "read".to_string(),
            description: "read a file".to_string(),
            schema: json!({"type": "object"}),
        }];
        let body = provider.request_body(&messages(), &tools);
        assert_eq!(body["model"], "gpt-5");
        assert_eq!(body["tools"][0]["function"]["name"], "read");
        assert_eq!(body["stream"], true);
    }
}
