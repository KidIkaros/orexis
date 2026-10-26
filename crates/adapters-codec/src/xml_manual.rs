use orexis_core::{Action, ToolSpec};
use orexis_ports::{Codec, Decoded};
use serde_json::{Map, Value};

/// Client-side XML in the model's trained dialect.
pub struct XmlManualCodec;

impl Codec for XmlManualCodec {
    fn name(&self) -> &'static str {
        "xml-manual"
    }

    fn prompt_fragment(&self, tools: &[ToolSpec]) -> String {
        format!(
            "Call tools with:\n<function name=\"TOOL\"><param name=\"ARG\">VALUE</param></function>\nFinish with:\n<function name=\"finish\"><param name=\"summary\">...</param></function>\nTools:\n{}",
            crate::tool_lines(tools)
        )
    }

    fn decode(&self, output: &str, _tools: &[ToolSpec]) -> Decoded {
        let mut actions = Vec::new();
        let mut errors = Vec::new();
        let mut cursor = 0usize;

        while let Some(relative) = output[cursor..].find("<function") {
            let start = cursor + relative;
            let before = output[cursor..start].trim();
            if !before.is_empty() {
                actions.push(Action::Say {
                    text: before.to_string(),
                });
            }
            let Some(relative_end) = output[start..].find("</function>") else {
                errors.push("unterminated <function> block".to_string());
                cursor = output.len();
                break;
            };
            let end = start + relative_end + "</function>".len();
            match parse_function(&output[start..end]) {
                Ok(action) => actions.push(action),
                Err(error) => errors.push(error),
            }
            cursor = end;
        }

        let tail = output[cursor..].trim();
        if !tail.is_empty() {
            actions.push(Action::Say {
                text: tail.to_string(),
            });
        }

        Decoded { actions, errors }
    }
}

fn parse_function(block: &str) -> Result<Action, String> {
    let open_end = block.find('>').ok_or("malformed <function> tag")?;
    let open = &block[..open_end];
    let name = attr(open, "name").ok_or("missing function name")?;

    let mut params = Map::new();
    let mut cursor = open_end + 1;
    while let Some(relative) = block[cursor..].find("<param") {
        let start = cursor + relative;
        let Some(relative_end) = block[start..].find("</param>") else {
            return Err("unterminated <param>".to_string());
        };
        let tag_end = start + block[start..].find('>').ok_or("malformed <param> tag")?;
        let tag = &block[start..tag_end];
        let key = attr(tag, "name").ok_or("missing param name")?;
        let value = &block[tag_end + 1..start + relative_end];
        params.insert(key, Value::String(value.trim().to_string()));
        cursor = start + relative_end + "</param>".len();
    }

    if name == "finish" || name == "done" {
        let summary = params
            .get("summary")
            .and_then(Value::as_str)
            .unwrap_or("done")
            .to_string();
        Ok(Action::Done { summary })
    } else {
        Ok(Action::CallTool {
            name,
            args: Value::Object(params).to_string(),
        })
    }
}

fn attr(tag: &str, name: &str) -> Option<String> {
    let needle = format!("{name}=\"");
    let start = tag.find(&needle)? + needle.len();
    let end = tag[start..].find('"')? + start;
    Some(tag[start..end].to_string())
}
