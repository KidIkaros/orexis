//! Anthropic messages adapter: tool blocks, cache breakpoints, SSE parsing.

use std::collections::BTreeMap;

use async_trait::async_trait;
use futures::StreamExt;
use orexis_adapters_http::{ByteStream, Transport};
use orexis_adapters_provider_catalog::ProviderProfile;
use orexis_core::{Action, Message, Role, ToolSpec};
use orexis_ports::Provider;
use serde_json::{json, Value};

pub struct AnthropicProvider<T: Transport> {
    profile: ProviderProfile,
    model: String,
    api_key: Option<String>,
    transport: T,
}

impl<T: Transport> AnthropicProvider<T> {
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

    fn max_tokens(&self) -> u32 {
        self.profile
            .models
            .iter()
            .find(|model| model.id == self.model)
            .map(|model| model.max_output)
            .filter(|max| *max > 0)
            .unwrap_or(4096)
    }

    pub fn request_body(&self, messages: &[Message], tools: &[ToolSpec]) -> Value {
        let system = messages
            .iter()
            .filter(|message| message.role == Role::System)
            .map(|message| message.text.as_str())
            .collect::<Vec<_>>()
            .join("\n\n");
        let messages: Vec<Value> = messages
            .iter()
            .filter(|message| message.role != Role::System)
            .map(|message| match message.role {
                Role::User => json!({ "role": "user", "content": message.text }),
                Role::Assistant => json!({ "role": "assistant", "content": message.text }),
                Role::Tool => json!({
                    "role": "user",
                    "content": format!("[tool result] {}", message.text),
                }),
                Role::System => json!({ "role": "user", "content": message.text }),
            })
            .collect();

        let mut body = json!({
            "model": self.model,
            "max_tokens": self.max_tokens(),
            "stream": true,
            "messages": messages,
        });
        if !system.is_empty() {
            body["system"] = json!(system);
        }

        if !tools.is_empty() {
            let mut tools_json: Vec<Value> = tools
                .iter()
                .map(|tool| {
                    json!({
                        "name": tool.name,
                        "description": tool.description,
                        "input_schema": tool.schema,
                    })
                })
                .collect();
            if self.profile.cache.breakpoints {
                if let Some(last) = tools_json.last_mut() {
                    last["cache_control"] = json!({ "type": "ephemeral" });
                }
            }
            body["tools"] = json!(tools_json);
        }

        body
    }
}

#[async_trait]
impl<T: Transport + 'static> Provider for AnthropicProvider<T> {
    async fn complete(
        &self,
        messages: &[Message],
        tools: &[ToolSpec],
        _step: usize,
    ) -> Vec<Action> {
        let url = format!(
            "{}/v1/messages",
            self.profile.base_url.trim_end_matches('/')
        );
        let mut headers = vec![
            ("content-type".to_string(), "application/json".to_string()),
            ("anthropic-version".to_string(), "2023-06-01".to_string()),
        ];
        if let Some(key) = &self.api_key {
            headers.push(("x-api-key".to_string(), key.clone()));
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
struct ToolAccum {
    name: String,
    json: String,
}

async fn parse_stream(mut stream: ByteStream) -> Vec<Action> {
    let mut buffer = String::new();
    let mut text = String::new();
    let mut tools: BTreeMap<usize, ToolAccum> = BTreeMap::new();
    let mut stop_reason: Option<String> = None;

    while let Some(chunk) = stream.next().await {
        let Ok(bytes) = chunk else {
            break;
        };
        buffer.push_str(&String::from_utf8_lossy(&bytes));

        while let Some(index) = buffer.find("\n\n") {
            let event = buffer[..index].to_string();
            buffer.drain(..index + 2);
            let data = event.lines().find_map(|line| line.strip_prefix("data: "));
            let Some(data) = data else {
                continue;
            };
            let Ok(value) = serde_json::from_str::<Value>(data) else {
                continue;
            };
            match value["type"].as_str() {
                Some("content_block_start") => {
                    let index = value["index"].as_u64().unwrap_or(0) as usize;
                    let block = &value["content_block"];
                    if block["type"] == "tool_use" {
                        let name = block["name"].as_str().unwrap_or_default().to_string();
                        tools.entry(index).or_default().name = name;
                    }
                }
                Some("content_block_delta") => {
                    let index = value["index"].as_u64().unwrap_or(0) as usize;
                    let delta = &value["delta"];
                    if let Some(fragment) = delta["text"].as_str() {
                        text.push_str(fragment);
                    }
                    if let Some(fragment) = delta["partial_json"].as_str() {
                        tools.entry(index).or_default().json.push_str(fragment);
                    }
                }
                Some("message_delta") => {
                    if let Some(reason) = value["delta"]["stop_reason"].as_str() {
                        stop_reason = Some(reason.to_string());
                    }
                }
                _ => {}
            }
        }
    }

    let mut actions = Vec::new();
    if !tools.is_empty() {
        for (_, tool) in tools {
            let args = if tool.json.is_empty() {
                "{}".to_string()
            } else {
                tool.json
            };
            actions.push(Action::CallTool {
                name: tool.name,
                args,
            });
        }
    } else if !text.is_empty() {
        actions.push(Action::Say { text: text.clone() });
        if stop_reason.as_deref() == Some("end_turn") {
            actions.push(Action::Done { summary: text });
        }
    } else if stop_reason.is_some() {
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

    fn profile() -> ProviderProfile {
        serde_yaml::from_str(include_str!(
            "../../adapters-provider-catalog/providers/anthropic.yaml"
        ))
        .expect("profile")
    }

    fn provider(chunks: Vec<&str>) -> AnthropicProvider<MockTransport> {
        AnthropicProvider::new(
            profile(),
            "claude-sonnet-4-5",
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
            "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"hel\"}}\n\n",
            "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"lo\"}}\n\n",
            "event: message_delta\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"}}\n\n",
            "event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n",
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
    async fn tool_use_becomes_call_tool() {
        let provider = provider(vec![
            "event: content_block_start\ndata: {\"type\":\"content_block_start\",\"index\":1,\"content_block\":{\"type\":\"tool_use\",\"id\":\"t1\",\"name\":\"read\"}}\n\n",
            "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":1,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"{\\\"path\\\":\"}}\n\n",
            "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":1,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"\\\"x\\\"}\"}}\n\n",
            "event: message_delta\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"tool_use\"}}\n\n",
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

    #[test]
    fn body_uses_anthropic_tool_shape_and_cache() {
        let provider = provider(vec![]);
        let tools = vec![ToolSpec {
            name: "read".to_string(),
            description: "read a file".to_string(),
            schema: json!({"type": "object"}),
        }];
        let body = provider.request_body(&messages(), &tools);
        assert_eq!(body["model"], "claude-sonnet-4-5");
        assert_eq!(body["max_tokens"], 64000);
        assert_eq!(body["tools"][0]["input_schema"]["type"], "object");
        assert_eq!(body["tools"][0]["cache_control"]["type"], "ephemeral");
    }

    #[test]
    fn system_messages_go_to_the_system_field() {
        let provider = provider(vec![]);
        let messages = vec![
            Message {
                role: Role::System,
                text: "be helpful".to_string(),
            },
            Message {
                role: Role::User,
                text: "hi".to_string(),
            },
        ];
        let body = provider.request_body(&messages, &[]);
        assert_eq!(body["system"], "be helpful");
        assert_eq!(body["messages"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn auth_uses_x_api_key() {
        let provider = provider(vec![
            "event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n",
        ]);
        let _ = provider.complete(&messages(), &[], 0).await;
        let headers = provider.transport().headers.lock().unwrap();
        assert!(headers
            .iter()
            .any(|(name, value)| name == "x-api-key" && value == "test-key"));
        assert!(headers.iter().any(|(name, _)| name == "anthropic-version"));
    }
}
