use super::*;

#[test]
fn tool_spec_defaults_a_missing_input_schema() {
    let spec = tool_spec("read_file".into(), "Read a file".into(), Value::Null);
    assert_eq!(spec.schema, serde_json::json!({"type": "object"}));
}

#[test]
fn tool_spec_keeps_a_present_schema() {
    let schema = serde_json::json!({"type": "object", "required": ["path"]});
    let spec = tool_spec("read_file".into(), String::new(), schema.clone());
    assert_eq!(spec.schema, schema);
}

#[test]
fn parse_tools_skips_nameless_entries() {
    let tools = parse_tools(&serde_json::json!({
        "tools": [
            {"name": "read_file", "description": "d", "inputSchema": {"type": "object"}},
            {"description": "no name"},
            {"name": "ping"}
        ]
    }));
    assert_eq!(tools.len(), 2);
    assert_eq!(tools[0].name, "read_file");
    assert_eq!(tools[1].name, "ping");
    assert_eq!(tools[1].schema, serde_json::json!({"type": "object"}));
}

#[test]
fn parse_resources_requires_a_uri() {
    let resources = parse_resources(&serde_json::json!({
        "resources": [
            {"uri": "file:///a", "name": "a", "mimeType": "text/plain"},
            {"name": "no uri"}
        ]
    }));
    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0].uri, "file:///a");
}

#[test]
fn parse_resource_contents_reads_text_and_blob() {
    let contents = parse_resource_contents(&serde_json::json!({
        "contents": [
            {"uri": "file:///a", "mimeType": "text/plain", "text": "body"},
            {"uri": "file:///b", "mimeType": "image/png", "blob": "base64"}
        ]
    }));
    assert_eq!(contents.len(), 2);
    assert_eq!(contents[0].text.as_deref(), Some("body"));
    assert_eq!(contents[1].blob.as_deref(), Some("base64"));
}

#[test]
fn parse_prompts_requires_a_name_and_defaults_arguments() {
    let prompts = parse_prompts(&serde_json::json!({
        "prompts": [
            {"name": "summarize", "description": "d",
             "arguments": [{"name": "text", "required": true}]},
            {"description": "no name"}
        ]
    }));
    assert_eq!(prompts.len(), 1);
    assert_eq!(prompts[0].arguments.len(), 1);
    assert!(prompts[0].arguments[0].required);
}

#[test]
fn parse_tool_content_maps_the_three_block_kinds() {
    let content = parse_tool_content(&serde_json::json!({
        "content": [
            {"type": "text", "text": "hi"},
            {"type": "image", "data": "base64", "mimeType": "image/png"},
            {"type": "resource", "resource": {"uri": "file:///a"}},
            {"type": "unknown"}
        ]
    }));
    assert_eq!(
        content,
        vec![
            ToolContent::Text { text: "hi".into() },
            ToolContent::Image {
                data: "base64".into(),
                mime_type: "image/png".into(),
            },
            ToolContent::Resource {
                resource: serde_json::json!({"uri": "file:///a"}),
            },
        ]
    );
}

#[test]
fn tool_content_text_extraction() {
    let text = ToolContent::Text {
        text: "hello".into(),
    };
    let image = ToolContent::Image {
        data: "base64".into(),
        mime_type: "image/png".into(),
    };
    assert_eq!(text.as_text(), Some("hello"));
    assert_eq!(image.as_text(), None);
}

#[test]
fn next_cursor_reads_and_ignores_empty() {
    assert_eq!(
        next_cursor(&serde_json::json!({"nextCursor": "p2"})),
        Some("p2".into())
    );
    assert_eq!(next_cursor(&serde_json::json!({"nextCursor": ""})), None);
    assert_eq!(next_cursor(&serde_json::json!({})), None);
}
