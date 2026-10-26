//! MCP protocol types ported from Kindred `src/mcp.rs`.

use serde_json::Value;

use orexis_core::ToolSpec;

#[derive(Debug, Clone, PartialEq)]
pub struct ClientInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}

/// Server capabilities from the initialize response. Raw values: the
/// capability objects carry per-feature options the core does not need.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ServerCapabilities {
    pub tools: Option<Value>,
    pub resources: Option<Value>,
    pub prompts: Option<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Initialization {
    pub protocol_version: String,
    pub server_info: ServerInfo,
    pub capabilities: ServerCapabilities,
}

/// A resource listing entry.
#[derive(Debug, Clone, PartialEq)]
pub struct McpResource {
    pub uri: String,
    pub name: String,
    pub mime_type: String,
    pub description: String,
}

/// A resource content block from resources/read: text or base64 blob.
#[derive(Debug, Clone, PartialEq)]
pub struct ResourceContent {
    pub uri: String,
    pub mime_type: String,
    pub text: Option<String>,
    pub blob: Option<String>,
}

/// A prompt template listing entry.
#[derive(Debug, Clone, PartialEq)]
pub struct McpPrompt {
    pub name: String,
    pub description: String,
    pub arguments: Vec<McpPromptArgument>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct McpPromptArgument {
    pub name: String,
    pub description: String,
    pub required: bool,
}

/// A content block returned by a tool call.
#[derive(Debug, Clone, PartialEq)]
pub enum ToolContent {
    Text { text: String },
    Image { data: String, mime_type: String },
    Resource { resource: Value },
}

impl ToolContent {
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text { text } => Some(text),
            _ => None,
        }
    }
}

/// The result of a tool call. `is_error` is the tool reporting failure
/// inside a successful JSON-RPC response (MCP tools/call semantics).
#[derive(Debug, Clone, PartialEq)]
pub struct ToolCallResult {
    pub success: bool,
    pub content: Vec<ToolContent>,
    pub is_error: bool,
}

/// An MCP tool maps onto the core `ToolSpec`; a missing inputSchema
/// defaults to an empty object schema.
pub fn tool_spec(name: String, description: String, input_schema: Value) -> ToolSpec {
    ToolSpec {
        name,
        description,
        schema: if input_schema.is_null() {
            serde_json::json!({"type": "object"})
        } else {
            input_schema
        },
    }
}

pub(crate) fn string_field(value: &Value, field: &str) -> String {
    value
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

pub(crate) fn parse_tools(result: &Value) -> Vec<ToolSpec> {
    result
        .get("tools")
        .and_then(Value::as_array)
        .map(|tools| {
            tools
                .iter()
                .filter_map(|tool| {
                    let name = tool.get("name")?.as_str()?.to_string();
                    Some(tool_spec(
                        name,
                        string_field(tool, "description"),
                        tool.get("inputSchema").cloned().unwrap_or(Value::Null),
                    ))
                })
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn next_cursor(result: &Value) -> Option<String> {
    result
        .get("nextCursor")
        .and_then(Value::as_str)
        .filter(|cursor| !cursor.is_empty())
        .map(str::to_string)
}

pub(crate) fn parse_resources(result: &Value) -> Vec<McpResource> {
    result
        .get("resources")
        .and_then(Value::as_array)
        .map(|resources| {
            resources
                .iter()
                .filter_map(|resource| {
                    let uri = resource.get("uri")?.as_str()?.to_string();
                    Some(McpResource {
                        uri,
                        name: string_field(resource, "name"),
                        mime_type: string_field(resource, "mimeType"),
                        description: string_field(resource, "description"),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn parse_resource_contents(result: &Value) -> Vec<ResourceContent> {
    result
        .get("contents")
        .and_then(Value::as_array)
        .map(|contents| {
            contents
                .iter()
                .filter_map(|content| {
                    let uri = content.get("uri")?.as_str()?.to_string();
                    Some(ResourceContent {
                        uri,
                        mime_type: string_field(content, "mimeType"),
                        text: content
                            .get("text")
                            .and_then(Value::as_str)
                            .map(str::to_string),
                        blob: content
                            .get("blob")
                            .and_then(Value::as_str)
                            .map(str::to_string),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn parse_prompts(result: &Value) -> Vec<McpPrompt> {
    result
        .get("prompts")
        .and_then(Value::as_array)
        .map(|prompts| {
            prompts
                .iter()
                .filter_map(|prompt| {
                    let name = prompt.get("name")?.as_str()?.to_string();
                    Some(McpPrompt {
                        name,
                        description: string_field(prompt, "description"),
                        arguments: prompt
                            .get("arguments")
                            .and_then(Value::as_array)
                            .map(|arguments| {
                                arguments
                                    .iter()
                                    .filter_map(|argument| {
                                        let name = argument.get("name")?.as_str()?.to_string();
                                        Some(McpPromptArgument {
                                            name,
                                            description: string_field(argument, "description"),
                                            required: argument
                                                .get("required")
                                                .and_then(Value::as_bool)
                                                .unwrap_or(false),
                                        })
                                    })
                                    .collect()
                            })
                            .unwrap_or_default(),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn parse_tool_content(result: &Value) -> Vec<ToolContent> {
    result
        .get("content")
        .and_then(Value::as_array)
        .map(|content| {
            content
                .iter()
                .filter_map(|block| match block.get("type").and_then(Value::as_str) {
                    Some("text") => Some(ToolContent::Text {
                        text: string_field(block, "text"),
                    }),
                    Some("image") => Some(ToolContent::Image {
                        data: string_field(block, "data"),
                        mime_type: string_field(block, "mimeType"),
                    }),
                    Some("resource") => Some(ToolContent::Resource {
                        resource: block.get("resource").cloned().unwrap_or(Value::Null),
                    }),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests;
