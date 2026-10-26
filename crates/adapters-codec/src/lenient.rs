use orexis_core::{Action, ToolSpec};
use orexis_ports::{Codec, Decoded};

/// Control/debug arm: fenced ```action blocks holding a JSON action.
pub struct LenientCodec;

impl Codec for LenientCodec {
    fn name(&self) -> &'static str {
        "lenient"
    }

    fn prompt_fragment(&self, tools: &[ToolSpec]) -> String {
        format!(
            "When you call a tool, write a fenced block:\n```action\n{{\"action\":\"call_tool\",\"name\":\"...\",\"args\":{{...}}}}\n```\nTools:\n{}",
            crate::tool_lines(tools)
        )
    }

    fn decode(&self, output: &str, _tools: &[ToolSpec]) -> Decoded {
        let mut actions = Vec::new();
        let mut errors = Vec::new();
        let mut cursor = 0usize;

        while let Some(relative) = output[cursor..].find("```action") {
            let start = cursor + relative;
            let before = output[cursor..start].trim();
            if !before.is_empty() {
                actions.push(Action::Say {
                    text: before.to_string(),
                });
            }

            let body_start = start + "```action".len();
            let Some(relative_close) = output[body_start..].find("```") else {
                errors.push("unterminated action block".to_string());
                cursor = output.len();
                break;
            };
            let body = output[body_start..body_start + relative_close].trim();
            match serde_json::from_str::<serde_json::Value>(body) {
                Ok(value) => {
                    let decoded = crate::decode_json_array(value);
                    actions.extend(decoded.actions);
                    errors.extend(decoded.errors);
                }
                Err(error) => errors.push(format!("invalid JSON block: {error}")),
            }
            cursor = body_start + relative_close + 3;
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
