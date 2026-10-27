//! Anthropic messages adapter: tool blocks, cache breakpoints, SSE parsing.

use std::collections::BTreeMap;

use async_trait::async_trait;
use futures::StreamExt;
use orexis_adapters_http::{ByteStream, Transport};
use orexis_adapters_provider_catalog::ProviderProfile;
use orexis_core::{Action, Message, ProviderResponse, Role, Thinking, ToolSpec};
use orexis_ports::{DeltaSink, NullDeltaSink, PortError, PortResult, Provider};
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

    pub fn request_body(
        &self,
        messages: &[Message],
        tools: &[ToolSpec],
        thinking: Thinking,
    ) -> Value {
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
                Role::Assistant => {
                    if message.tool_calls.is_empty() {
                        json!({ "role": "assistant", "content": message.text })
                    } else {
                        let mut blocks = Vec::new();
                        if !message.text.is_empty() {
                            blocks.push(json!({ "type": "text", "text": message.text }));
                        }
                        for call in &message.tool_calls {
                            let input = serde_json::from_str::<Value>(&call.args)
                                .unwrap_or_else(|_| json!({}));
                            blocks.push(json!({
                                "type": "tool_use",
                                "id": call.call_id,
                                "name": call.name,
                                "input": input
                            }));
                        }
                        json!({ "role": "assistant", "content": blocks })
                    }
                }
                Role::Tool => match &message.tool_call_id {
                    Some(call_id) => json!({
                        "role": "user",
                        "content": [{
                            "type": "tool_result",
                            "tool_use_id": call_id,
                            "content": message.text
                        }]
                    }),
                    None => json!({
                        "role": "user",
                        "content": format!("[tool result] {}", message.text),
                    }),
                },
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

        // Per-provider conditioning (PLAN §4.4): the profile declares the
        // parameter name and the level vocabulary; the anthropic wire
        // takes an enabled flag plus a token budget, so the adapter scales
        // the budget with the declared level's position. Off and Auto omit
        // the parameter — anthropic's default is off, so omission is the
        // honest minimum. On is the strongest declared level.
        if self.profile.thinking.supported && !self.profile.thinking.levels.is_empty() {
            let budget = match thinking {
                Thinking::Off | Thinking::Auto => None,
                Thinking::On => Some(self.profile.thinking.levels.len() - 1),
                Thinking::Level(index) => Some(index.min(self.profile.thinking.levels.len() - 1)),
            }
            .map(|index| 1024u32 << (index + 1));
            if let Some(budget) = budget {
                body["thinking"] = json!({ "type": "enabled", "budget_tokens": budget });
                // The wire requires max_tokens to exceed the budget.
                let max_tokens = self.max_tokens().max(budget + 1024);
                body["max_tokens"] = json!(max_tokens);
            }
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
        thinking: Thinking,
    ) -> PortResult<ProviderResponse> {
        request(self, messages, tools, thinking, &NullDeltaSink).await
    }

    async fn stream(
        &self,
        messages: &[Message],
        tools: &[ToolSpec],
        _step: usize,
        thinking: Thinking,
        sink: &dyn DeltaSink,
    ) -> PortResult<ProviderResponse> {
        request(self, messages, tools, thinking, sink).await
    }
}

async fn request<T: Transport>(
    provider: &AnthropicProvider<T>,
    messages: &[Message],
    tools: &[ToolSpec],
    thinking: Thinking,
    sink: &dyn DeltaSink,
) -> PortResult<ProviderResponse> {
    let url = format!(
        "{}/v1/messages",
        provider.profile.base_url.trim_end_matches('/')
    );
    let mut headers = vec![
        ("content-type".to_string(), "application/json".to_string()),
        ("anthropic-version".to_string(), "2023-06-01".to_string()),
    ];
    if let Some(key) = &provider.api_key {
        headers.push((
            provider.profile.auth.header.clone(),
            format!("{}{}", provider.profile.auth.prefix, key),
        ));
    }

    match provider
        .transport
        .post_stream(
            &url,
            headers,
            provider.request_body(messages, tools, thinking).to_string(),
        )
        .await
    {
        Ok(stream) => {
            let actions = parse_stream(stream, sink).await;
            let raw_text = actions
                .iter()
                .filter_map(|action| match action {
                    Action::Say { text } => Some(text.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("");
            Ok(ProviderResponse { actions, raw_text })
        }
        Err(error) => Err(PortError(error.to_string())),
    }
}

#[derive(Default)]
struct ToolAccum {
    id: String,
    name: String,
    json: String,
}

async fn parse_stream(mut stream: ByteStream, sink: &dyn DeltaSink) -> Vec<Action> {
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
                        let entry = tools.entry(index).or_default();
                        entry.name = name;
                        if let Some(id) = block["id"].as_str() {
                            entry.id = id.to_string();
                        }
                    }
                }
                Some("content_block_delta") => {
                    let index = value["index"].as_u64().unwrap_or(0) as usize;
                    let delta = &value["delta"];
                    if let Some(fragment) = delta["text"].as_str() {
                        text.push_str(fragment);
                        sink.delta(fragment);
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
        for (index, tool) in tools {
            let args = if tool.json.is_empty() {
                "{}".to_string()
            } else {
                tool.json
            };
            actions.push(Action::CallTool {
                call_id: if tool.id.is_empty() {
                    format!("tool_{index}")
                } else {
                    tool.id
                },
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
            tool_call_id: None,
            tool_calls: Vec::new(),
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
        let actions = provider
            .complete(&messages(), &[], 0, Thinking::Auto)
            .await
            .expect("complete")
            .actions;
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
        let actions = provider
            .complete(&messages(), &[], 0, Thinking::Auto)
            .await
            .expect("complete")
            .actions;
        assert_eq!(
            actions,
            vec![Action::CallTool {
                call_id: "t1".to_string(),
                name: "read".to_string(),
                args: "{\"path\":\"x\"}".to_string()
            }]
        );
    }

    #[tokio::test]
    async fn transport_error_is_returned() {
        use orexis_adapters_http::FailingTransport;
        let provider = AnthropicProvider::new(
            profile(),
            "claude-sonnet-4-5",
            None,
            FailingTransport {
                message: "boom".to_string(),
            },
        );
        let result = provider.complete(&messages(), &[], 0, Thinking::Auto).await;
        assert!(result.is_err());
    }

    #[test]
    fn assistant_tool_use_and_tool_results_are_paired() {
        use orexis_core::ToolCall;
        let provider = provider(vec![]);
        let messages = vec![
            Message {
                role: Role::Assistant,
                text: String::new(),
                tool_call_id: None,
                tool_calls: vec![ToolCall {
                    call_id: "t1".to_string(),
                    name: "read".to_string(),
                    args: "{\"path\":\"x\"}".to_string(),
                }],
            },
            Message {
                role: Role::Tool,
                text: "read: ok".to_string(),
                tool_call_id: Some("t1".to_string()),
                tool_calls: Vec::new(),
            },
        ];
        let body = provider.request_body(&messages, &[], Thinking::Auto);
        assert_eq!(body["messages"][0]["content"][0]["type"], "tool_use");
        assert_eq!(body["messages"][0]["content"][0]["id"], "t1");
        assert_eq!(body["messages"][1]["content"][0]["type"], "tool_result");
        assert_eq!(body["messages"][1]["content"][0]["tool_use_id"], "t1");
    }

    #[test]
    fn body_uses_anthropic_tool_shape_and_cache() {
        let provider = provider(vec![]);
        let tools = vec![ToolSpec {
            name: "read".to_string(),
            description: "read a file".to_string(),
            schema: json!({"type": "object"}),
        }];
        let body = provider.request_body(&messages(), &tools, Thinking::Auto);
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
                tool_call_id: None,
                tool_calls: Vec::new(),
            },
            Message {
                role: Role::User,
                text: "hi".to_string(),
                tool_call_id: None,
                tool_calls: Vec::new(),
            },
        ];
        let body = provider.request_body(&messages, &[], Thinking::Auto);
        assert_eq!(body["system"], "be helpful");
        assert_eq!(body["messages"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn auth_header_uses_the_profile() {
        let mut profile = profile();
        profile.auth.header = "authorization".to_string();
        profile.auth.prefix = "Bearer ".to_string();
        let provider = AnthropicProvider::new(
            profile,
            "claude-sonnet-4-5",
            Some("test-key".to_string()),
            MockTransport::new(vec![
                "event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n".to_string(),
            ]),
        );
        let _ = provider.complete(&messages(), &[], 0, Thinking::Auto).await;
        let headers = provider.transport().headers.lock().unwrap();
        assert!(headers
            .iter()
            .any(|(name, value)| name == "authorization" && value == "Bearer test-key"));
    }

    #[tokio::test]
    async fn auth_uses_x_api_key() {
        let provider = provider(vec![
            "event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n",
        ]);
        let _ = provider.complete(&messages(), &[], 0, Thinking::Auto).await;
        let headers = provider.transport().headers.lock().unwrap();
        assert!(headers
            .iter()
            .any(|(name, value)| name == "x-api-key" && value == "test-key"));
        assert!(headers.iter().any(|(name, _)| name == "anthropic-version"));
    }

    struct RecordingDeltaSink {
        deltas: std::sync::Mutex<Vec<String>>,
    }

    impl DeltaSink for RecordingDeltaSink {
        fn delta(&self, text: &str) {
            self.deltas.lock().expect("deltas").push(text.to_string());
        }
    }

    #[tokio::test]
    async fn stream_emits_content_deltas() {
        let provider = provider(vec![
            "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"hel\"}}\n\n",
            "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"lo\"}}\n\n",
            "event: message_delta\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"}}\n\n",
        ]);
        let sink = RecordingDeltaSink {
            deltas: std::sync::Mutex::new(Vec::new()),
        };
        let actions = provider
            .stream(&messages(), &[], 0, Thinking::Auto, &sink)
            .await
            .expect("stream")
            .actions;
        assert_eq!(
            sink.deltas.lock().expect("deltas").as_slice(),
            ["hel", "lo"]
        );
        assert!(matches!(actions[0], Action::Say { .. }));
    }
}
