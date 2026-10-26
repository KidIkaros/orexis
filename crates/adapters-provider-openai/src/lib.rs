//! OpenAI-compatible provider adapter: request building and SSE parsing.

use std::collections::BTreeMap;

use async_trait::async_trait;
use futures::StreamExt;
use orexis_adapters_http::{ByteStream, Transport};
use orexis_adapters_provider_catalog::ProviderProfile;
use orexis_core::{Action, Message, ProviderResponse, Role, ToolSpec};
use orexis_ports::{DeltaSink, NullDeltaSink, PortError, PortResult, Provider};
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
                Role::Assistant => {
                    if message.tool_calls.is_empty() {
                        json!({ "role": "assistant", "content": message.text })
                    } else {
                        json!({
                            "role": "assistant",
                            "content": message.text,
                            "tool_calls": message
                                .tool_calls
                                .iter()
                                .map(|call| json!({
                                    "id": call.call_id,
                                    "type": "function",
                                    "function": { "name": call.name, "arguments": call.args }
                                }))
                                .collect::<Vec<_>>()
                        })
                    }
                }
                Role::Tool => match &message.tool_call_id {
                    Some(call_id) => json!({
                        "role": "tool",
                        "tool_call_id": call_id,
                        "content": message.text,
                    }),
                    None => json!({
                        "role": "user",
                        "content": format!("[tool result] {}", message.text),
                    }),
                },
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
    ) -> PortResult<ProviderResponse> {
        request(self, messages, tools, &NullDeltaSink).await
    }

    async fn stream(
        &self,
        messages: &[Message],
        tools: &[ToolSpec],
        _step: usize,
        sink: &dyn DeltaSink,
    ) -> PortResult<ProviderResponse> {
        request(self, messages, tools, sink).await
    }
}

async fn request<T: Transport>(
    provider: &OpenAiCompatibleProvider<T>,
    messages: &[Message],
    tools: &[ToolSpec],
    sink: &dyn DeltaSink,
) -> PortResult<ProviderResponse> {
    let url = format!(
        "{}/chat/completions",
        provider.profile.base_url.trim_end_matches('/')
    );
    let mut headers = vec![("content-type".to_string(), "application/json".to_string())];
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
            provider.request_body(messages, tools).to_string(),
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
struct ToolCallAccum {
    id: String,
    name: String,
    args: String,
}

async fn parse_stream(mut stream: ByteStream, sink: &dyn DeltaSink) -> Vec<Action> {
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
                    sink.delta(text);
                }
                if let Some(calls) = choice["delta"]["tool_calls"].as_array() {
                    for call in calls {
                        let index = call["index"].as_u64().unwrap_or(0) as usize;
                        let entry = tool_calls.entry(index).or_default();
                        if let Some(id) = call["id"].as_str() {
                            entry.id = id.to_string();
                        }
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
        for (index, call) in tool_calls {
            actions.push(Action::CallTool {
                call_id: if call.id.is_empty() {
                    format!("call_{index}")
                } else {
                    call.id
                },
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
            tool_call_id: None,
            tool_calls: Vec::new(),
        }]
    }

    #[tokio::test]
    async fn content_stream_becomes_say_and_done() {
        let provider = provider(vec![
            "data: {\"choices\":[{\"delta\":{\"content\":\"hel\"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"lo\"},\"finish_reason\":\"stop\"}]}\n\n",
            "data: [DONE]\n\n",
        ]);
        let actions = provider
            .complete(&messages(), &[], 0)
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
    async fn tool_call_stream_becomes_call_tool() {
        let provider = provider(vec![
            "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"name\":\"read\",\"arguments\":\"{\\\"path\\\":\"}}]}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"arguments\":\"\\\"x\\\"}\"}}]}}]}\n\n",
            "data: [DONE]\n\n",
        ]);
        let actions = provider
            .complete(&messages(), &[], 0)
            .await
            .expect("complete")
            .actions;
        assert_eq!(
            actions,
            vec![Action::CallTool {
                call_id: "call_0".to_string(),
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

    #[tokio::test]
    async fn transport_error_is_returned() {
        use orexis_adapters_http::FailingTransport;
        let provider = OpenAiCompatibleProvider::new(
            profile(),
            "gpt-5",
            None,
            FailingTransport {
                message: "boom".to_string(),
            },
        );
        let result = provider.complete(&messages(), &[], 0).await;
        assert!(result.is_err());
    }

    #[test]
    fn assistant_tool_calls_and_tool_results_are_paired() {
        use orexis_core::ToolCall;
        let provider = provider(vec![]);
        let messages = vec![
            Message {
                role: Role::Assistant,
                text: "reading".to_string(),
                tool_call_id: None,
                tool_calls: vec![ToolCall {
                    call_id: "call_1".to_string(),
                    name: "read".to_string(),
                    args: "{\"path\":\"x\"}".to_string(),
                }],
            },
            Message {
                role: Role::Tool,
                text: "read: ok".to_string(),
                tool_call_id: Some("call_1".to_string()),
                tool_calls: Vec::new(),
            },
        ];
        let body = provider.request_body(&messages, &[]);
        assert_eq!(body["messages"][0]["tool_calls"][0]["id"], "call_1");
        assert_eq!(body["messages"][1]["role"], "tool");
        assert_eq!(body["messages"][1]["tool_call_id"], "call_1");
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
            "data: {\"choices\":[{\"delta\":{\"content\":\"hel\"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"lo\"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n",
            "data: [DONE]\n\n",
        ]);
        let sink = RecordingDeltaSink {
            deltas: std::sync::Mutex::new(Vec::new()),
        };
        let actions = provider
            .stream(&messages(), &[], 0, &sink)
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
