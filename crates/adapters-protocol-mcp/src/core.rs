//! The MCP client state machine: Start → Initializing → Ready.
//!
//! The caller drives everything: `initialize`/`list_tools`/… enqueue
//! outgoing messages, `pop_outgoing` drains them, `receive` consumes one
//! incoming line and returns the events it produces. Server-initiated
//! requests become `ServerRequest` events the caller answers with
//! `respond`/`refuse` — the core never blocks, so interleaved traffic
//! cannot deadlock it.

use std::collections::VecDeque;

use orexis_adapters_jsonrpc::{
    Correlations, ErrorObject, Message, Notification, Request, RequestId, Response,
};
use serde_json::Value;

use orexis_core::ToolSpec;

use crate::error::McpError;
use crate::types::{
    next_cursor, parse_prompts, parse_resource_contents, parse_resources, parse_tool_content,
    parse_tools, string_field, ClientInfo, Initialization, McpPrompt, McpResource, ResourceContent,
    ServerCapabilities, ServerInfo, ToolCallResult,
};

pub const PROTOCOL_VERSION: &str = "2025-06-18";

#[derive(Debug, Clone, PartialEq)]
enum Pending {
    Initialize,
    ToolsList,
    ToolCall,
    ResourcesList,
    ResourceRead,
    PromptsList,
    PromptGet,
    Ping,
}

impl Pending {
    fn method(&self) -> &'static str {
        match self {
            Self::Initialize => "initialize",
            Self::ToolsList => "tools/list",
            Self::ToolCall => "tools/call",
            Self::ResourcesList => "resources/list",
            Self::ResourceRead => "resources/read",
            Self::PromptsList => "prompts/list",
            Self::PromptGet => "prompts/get",
            Self::Ping => "ping",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Start,
    Initializing,
    Ready,
}

/// Events the core produces from incoming traffic.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Initialized(Initialization),
    ToolsListed {
        tools: Vec<ToolSpec>,
        next_cursor: Option<String>,
    },
    ToolCalled(ToolCallResult),
    ResourcesListed {
        resources: Vec<McpResource>,
        next_cursor: Option<String>,
    },
    ResourceRead {
        contents: Vec<ResourceContent>,
    },
    PromptsListed {
        prompts: Vec<McpPrompt>,
        next_cursor: Option<String>,
    },
    PromptGot(Value),
    Pong,
    Failed {
        method: String,
        error: ErrorObject,
    },
    ServerRequest {
        id: RequestId,
        method: String,
        params: Option<Value>,
    },
    ServerNotification {
        method: String,
        params: Option<Value>,
    },
    ToolsChanged,
    ResourcesChanged,
    PromptsChanged,
    Stray(Response),
}

#[derive(Debug)]
pub struct McpCore {
    state: State,
    client_info: ClientInfo,
    next_id: i64,
    pending: Correlations<Pending>,
    outgoing: VecDeque<Message>,
}

impl McpCore {
    pub fn new(client_info: ClientInfo) -> Self {
        Self {
            state: State::Start,
            client_info,
            next_id: 1,
            pending: Correlations::new(),
            outgoing: VecDeque::new(),
        }
    }

    pub fn pop_outgoing(&mut self) -> Option<Message> {
        self.outgoing.pop_front()
    }

    fn send(&mut self, pending: Pending, params: Option<Value>) {
        let id = RequestId::Number(self.next_id);
        self.next_id += 1;
        self.pending.insert(id.clone(), pending.clone());
        self.outgoing.push_back(Message::Request(Request {
            id,
            method: pending.method().to_string(),
            params,
        }));
    }

    pub fn initialize(&mut self) -> Result<(), McpError> {
        if self.state != State::Start {
            return Err(McpError::AlreadyInitializing);
        }
        self.state = State::Initializing;
        self.send(
            Pending::Initialize,
            Some(serde_json::json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": {
                    "name": self.client_info.name,
                    "version": self.client_info.version,
                },
            })),
        );
        Ok(())
    }

    fn require_ready(&self) -> Result<(), McpError> {
        if self.state == State::Ready {
            Ok(())
        } else {
            Err(McpError::NotReady)
        }
    }

    pub fn list_tools(&mut self, cursor: Option<String>) -> Result<(), McpError> {
        self.require_ready()?;
        self.send(
            Pending::ToolsList,
            cursor.map(|cursor| serde_json::json!({"cursor": cursor})),
        );
        Ok(())
    }

    pub fn call_tool(&mut self, name: &str, arguments: Option<Value>) -> Result<(), McpError> {
        self.require_ready()?;
        let mut params = serde_json::json!({"name": name});
        if let Some(arguments) = arguments {
            params["arguments"] = arguments;
        }
        self.send(Pending::ToolCall, Some(params));
        Ok(())
    }

    pub fn list_resources(&mut self, cursor: Option<String>) -> Result<(), McpError> {
        self.require_ready()?;
        self.send(
            Pending::ResourcesList,
            cursor.map(|cursor| serde_json::json!({"cursor": cursor})),
        );
        Ok(())
    }

    pub fn read_resource(&mut self, uri: &str) -> Result<(), McpError> {
        self.require_ready()?;
        self.send(Pending::ResourceRead, Some(serde_json::json!({"uri": uri})));
        Ok(())
    }

    pub fn list_prompts(&mut self, cursor: Option<String>) -> Result<(), McpError> {
        self.require_ready()?;
        self.send(
            Pending::PromptsList,
            cursor.map(|cursor| serde_json::json!({"cursor": cursor})),
        );
        Ok(())
    }

    pub fn get_prompt(&mut self, name: &str, arguments: Option<Value>) -> Result<(), McpError> {
        self.require_ready()?;
        let mut params = serde_json::json!({"name": name});
        if let Some(arguments) = arguments {
            params["arguments"] = arguments;
        }
        self.send(Pending::PromptGet, Some(params));
        Ok(())
    }

    pub fn ping(&mut self) -> Result<(), McpError> {
        self.require_ready()?;
        self.send(Pending::Ping, None);
        Ok(())
    }

    pub fn respond(&mut self, id: RequestId, result: Value) {
        self.outgoing
            .push_back(Message::Response(Response::result(id, result)));
    }

    pub fn refuse(&mut self, id: RequestId) {
        self.outgoing.push_back(Message::Response(Response::error(
            Some(id),
            ErrorObject {
                code: -32601,
                message: "method not found".into(),
                data: None,
            },
        )));
    }

    pub fn receive(&mut self, line: &str) -> Result<Vec<Event>, McpError> {
        let message = orexis_adapters_jsonrpc::parse_line(line)?;
        Ok(match message {
            Message::Response(response) => self.on_response(response),
            Message::Request(request) => vec![Event::ServerRequest {
                id: request.id,
                method: request.method,
                params: request.params,
            }],
            Message::Notification(notification) => self.on_notification(notification),
        })
    }

    fn on_response(&mut self, response: Response) -> Vec<Event> {
        let Response { id, result, error } = response;
        let Some(id) = id else {
            return vec![Event::Stray(Response {
                id: None,
                result,
                error,
            })];
        };
        let Some(pending) = self.pending.take(&id) else {
            return vec![Event::Stray(Response {
                id: Some(id),
                result,
                error,
            })];
        };
        let Some(result) = result else {
            let error = error.unwrap_or(ErrorObject {
                code: -32603,
                message: "response without result or error".into(),
                data: None,
            });
            return vec![Event::Failed {
                method: pending.method().to_string(),
                error,
            }];
        };
        match pending {
            Pending::Initialize => self.on_initialized(result),
            Pending::ToolsList => vec![Event::ToolsListed {
                tools: parse_tools(&result),
                next_cursor: next_cursor(&result),
            }],
            Pending::ToolCall => {
                let is_error = result
                    .get("isError")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                vec![Event::ToolCalled(ToolCallResult {
                    success: !is_error,
                    content: parse_tool_content(&result),
                    is_error,
                })]
            }
            Pending::ResourcesList => vec![Event::ResourcesListed {
                resources: parse_resources(&result),
                next_cursor: next_cursor(&result),
            }],
            Pending::ResourceRead => vec![Event::ResourceRead {
                contents: parse_resource_contents(&result),
            }],
            Pending::PromptsList => vec![Event::PromptsListed {
                prompts: parse_prompts(&result),
                next_cursor: next_cursor(&result),
            }],
            Pending::PromptGet => vec![Event::PromptGot(result)],
            Pending::Ping => vec![Event::Pong],
        }
    }

    fn on_initialized(&mut self, result: Value) -> Vec<Event> {
        let protocol_version = string_field(&result, "protocolVersion");
        if protocol_version.is_empty() {
            self.state = State::Start;
            return vec![Event::Failed {
                method: Pending::Initialize.method().to_string(),
                error: ErrorObject {
                    code: -32603,
                    message: "initialize result without a protocol version".into(),
                    data: None,
                },
            }];
        }
        let server_info = result.get("serverInfo").cloned().unwrap_or(Value::Null);
        let event = Event::Initialized(Initialization {
            protocol_version,
            server_info: ServerInfo {
                name: string_field(&server_info, "name"),
                version: string_field(&server_info, "version"),
            },
            capabilities: ServerCapabilities {
                tools: result
                    .get("capabilities")
                    .and_then(|caps| caps.get("tools"))
                    .cloned(),
                resources: result
                    .get("capabilities")
                    .and_then(|caps| caps.get("resources"))
                    .cloned(),
                prompts: result
                    .get("capabilities")
                    .and_then(|caps| caps.get("prompts"))
                    .cloned(),
            },
        });
        self.state = State::Ready;
        self.outgoing.push_back(Message::Notification(Notification {
            method: "notifications/initialized".to_string(),
            params: None,
        }));
        vec![event]
    }

    fn on_notification(&mut self, notification: Notification) -> Vec<Event> {
        let event = match notification.method.as_str() {
            "notifications/tools/list_changed" => Event::ToolsChanged,
            "notifications/resources/list_changed" => Event::ResourcesChanged,
            "notifications/prompts/list_changed" => Event::PromptsChanged,
            _ => Event::ServerNotification {
                method: notification.method,
                params: notification.params,
            },
        };
        vec![event]
    }
}

#[cfg(test)]
mod tests;
