use orexis_core::ToolSpec;
use orexis_ports::{Codec, Decoded};

/// The native arm is a marker: the provider adapter already turned tool calls
/// into actions, so there is no text to parse.
pub struct NativeCodec;

impl Codec for NativeCodec {
    fn name(&self) -> &'static str {
        "native"
    }

    fn prompt_fragment(&self, _tools: &[ToolSpec]) -> String {
        String::new()
    }

    fn decode(&self, _output: &str, _tools: &[ToolSpec]) -> Decoded {
        Decoded {
            actions: Vec::new(),
            errors: Vec::new(),
        }
    }

    fn uses_native_tools(&self) -> bool {
        true
    }
}
