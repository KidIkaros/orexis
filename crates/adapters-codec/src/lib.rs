//! The four codecs (ADR-0003): native, xml-manual, json, lenient.
//! Sans-io and pure: decode text into actions, never touch a provider.

use orexis_core::{Action, CodecId, ToolSpec};
use orexis_ports::{Codec, Decoded};
use serde_json::Value;

pub mod json;
pub mod lenient;
pub mod native;
pub mod xml_manual;

pub use json::JsonCodec;
pub use lenient::LenientCodec;
pub use native::NativeCodec;
pub use xml_manual::XmlManualCodec;

pub fn codec_for(id: CodecId) -> Box<dyn Codec> {
    match id {
        CodecId::Native => Box::new(NativeCodec),
        CodecId::XmlManual => Box::new(XmlManualCodec),
        CodecId::Json => Box::new(JsonCodec),
        CodecId::Lenient => Box::new(LenientCodec),
    }
}

fn tool_lines(tools: &[ToolSpec]) -> String {
    tools
        .iter()
        .map(|tool| format!("- {}: {}", tool.name, tool.description))
        .collect::<Vec<_>>()
        .join("\n")
}

fn parse_json_action(value: Value) -> Result<Action, String> {
    let action = value
        .get("action")
        .and_then(Value::as_str)
        .ok_or_else(|| "missing action".to_string())?;
    match action {
        "say" => Ok(Action::Say {
            text: value
                .get("text")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        }),
        "done" => Ok(Action::Done {
            summary: value
                .get("summary")
                .and_then(Value::as_str)
                .unwrap_or("done")
                .to_string(),
        }),
        "call_tool" => {
            let name = value
                .get("name")
                .and_then(Value::as_str)
                .ok_or_else(|| "missing tool name".to_string())?;
            let args = value
                .get("args")
                .cloned()
                .unwrap_or_else(|| Value::Object(Default::default()));
            Ok(Action::CallTool {
                call_id: String::new(),
                name: name.to_string(),
                args: args.to_string(),
            })
        }
        other => Err(format!("unknown action '{other}'")),
    }
}

fn decode_json_array(value: Value) -> Decoded {
    match value {
        Value::Array(items) => {
            let mut actions = Vec::new();
            let mut errors = Vec::new();
            for item in items {
                match parse_json_action(item) {
                    Ok(action) => actions.push(action),
                    Err(error) => errors.push(error),
                }
            }
            Decoded { actions, errors }
        }
        other => match parse_json_action(other) {
            Ok(action) => Decoded {
                actions: vec![action],
                errors: Vec::new(),
            },
            Err(error) => Decoded {
                actions: Vec::new(),
                errors: vec![error],
            },
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tools() -> Vec<ToolSpec> {
        vec![ToolSpec {
            name: "read".to_string(),
            description: "read a file".to_string(),
            schema: serde_json::json!({"type": "object"}),
        }]
    }

    #[test]
    fn native_is_a_marker_codec() {
        let codec = NativeCodec;
        assert!(codec.uses_native_tools());
        assert!(codec.prompt_fragment(&tools()).is_empty());
        let decoded = codec.decode("ignored", &tools());
        assert!(decoded.actions.is_empty());
        assert!(decoded.errors.is_empty());
    }

    #[test]
    fn xml_decodes_calls_text_and_finish() {
        let codec = XmlManualCodec;
        let output = "I'll read it.\n<function name=\"read\"><param name=\"path\">src/lib.rs</param></function>\n<function name=\"finish\"><param name=\"summary\">done</param></function>";
        let decoded = codec.decode(output, &tools());
        assert_eq!(decoded.errors, Vec::<String>::new());
        assert_eq!(decoded.actions.len(), 3);
        assert!(matches!(decoded.actions[0], Action::Say { .. }));
        assert!(matches!(
            &decoded.actions[1],
            Action::CallTool { name, .. } if name == "read"
        ));
        assert!(matches!(
            &decoded.actions[2],
            Action::Done { summary } if summary == "done"
        ));
    }

    #[test]
    fn xml_decodes_multiple_calls_in_order() {
        let codec = XmlManualCodec;
        let output = "<function name=\"read\"><param name=\"path\">a</param></function><function name=\"read\"><param name=\"path\">b</param></function>";
        let decoded = codec.decode(output, &tools());
        assert_eq!(decoded.actions.len(), 2);
        assert!(
            matches!(&decoded.actions[0], Action::CallTool { args, .. } if args.contains("\"a\""))
        );
        assert!(
            matches!(&decoded.actions[1], Action::CallTool { args, .. } if args.contains("\"b\""))
        );
    }

    #[test]
    fn xml_reports_malformed_blocks() {
        let codec = XmlManualCodec;
        let decoded = codec.decode("<function name=\"read\">", &tools());
        assert_eq!(decoded.actions.len(), 0);
        assert_eq!(decoded.errors.len(), 1);
    }

    #[test]
    fn json_decodes_actions() {
        let codec = JsonCodec;
        let decoded = codec.decode(
            r#"{"action":"call_tool","name":"read","args":{"path":"x"}}"#,
            &tools(),
        );
        assert_eq!(decoded.errors, Vec::<String>::new());
        assert!(matches!(
            &decoded.actions[0],
            Action::CallTool { name, args, .. } if name == "read" && args.contains("\"x\"")
        ));
    }

    #[test]
    fn json_decodes_arrays() {
        let codec = JsonCodec;
        let decoded = codec.decode(
            r#"[{"action":"say","text":"hi"},{"action":"done","summary":"ok"}]"#,
            &tools(),
        );
        assert_eq!(decoded.actions.len(), 2);
        assert!(matches!(&decoded.actions[1], Action::Done { summary } if summary == "ok"));
    }

    #[test]
    fn json_reports_invalid_input() {
        let codec = JsonCodec;
        let decoded = codec.decode("not json", &tools());
        assert!(decoded.actions.is_empty());
        assert_eq!(decoded.errors.len(), 1);
    }

    #[test]
    fn lenient_decodes_fenced_blocks() {
        let codec = LenientCodec;
        let output =
            "thinking\n```action\n{\"action\":\"call_tool\",\"name\":\"read\",\"args\":{}}\n```\n";
        let decoded = codec.decode(output, &tools());
        assert!(matches!(
            &decoded.actions[0],
            Action::Say { text } if text == "thinking"
        ));
        assert!(matches!(
            &decoded.actions[1],
            Action::CallTool { name, .. } if name == "read"
        ));
    }

    #[test]
    fn lenient_reports_malformed_blocks() {
        let codec = LenientCodec;
        let decoded = codec.decode("```action\nnot json\n```", &tools());
        assert!(decoded.actions.is_empty());
        assert_eq!(decoded.errors.len(), 1);
    }

    #[test]
    fn json_grammar_is_present() {
        let codec = JsonCodec;
        assert!(codec.grammar(&tools()).is_some());
    }
}
