use orexis_core::ToolSpec;
use orexis_ports::{Codec, Decoded};

/// Grammar-constrained JSON actions: a flat object with an `action` field.
pub struct JsonCodec;

impl Codec for JsonCodec {
    fn name(&self) -> &'static str {
        "json"
    }

    fn prompt_fragment(&self, tools: &[ToolSpec]) -> String {
        format!(
            "Respond with one JSON action:\n{{\"action\":\"say\",\"text\":\"...\"}}\n{{\"action\":\"call_tool\",\"name\":\"...\",\"args\":{{...}}}}\n{{\"action\":\"done\",\"summary\":\"...\"}}\nTools:\n{}",
            crate::tool_lines(tools)
        )
    }

    fn decode(&self, output: &str, _tools: &[ToolSpec]) -> Decoded {
        match serde_json::from_str::<serde_json::Value>(output.trim()) {
            Ok(value) => crate::decode_json_array(value),
            Err(error) => Decoded {
                actions: Vec::new(),
                errors: vec![format!("invalid JSON: {error}")],
            },
        }
    }

    fn grammar(&self, _tools: &[ToolSpec]) -> Option<String> {
        Some(
            r#"{"type":"object","required":["action"],"properties":{"action":{"enum":["say","call_tool","done"]},"text":{"type":"string"},"name":{"type":"string"},"args":{"type":"object"},"summary":{"type":"string"}}}"#
                .to_string(),
        )
    }
}
