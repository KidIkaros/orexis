//! Codec selection (ADR-0003).

use orexis_core::CodecId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodecCapabilities {
    pub native_tools: bool,
    pub xml_dialect: bool,
    pub grammar: bool,
}

/// ADR-0003: native → known XML dialect → grammar JSON → lenient.
pub fn select_codec(capabilities: CodecCapabilities, override_id: Option<CodecId>) -> CodecId {
    if let Some(id) = override_id {
        return id;
    }
    if capabilities.native_tools {
        CodecId::Native
    } else if capabilities.xml_dialect {
        CodecId::XmlManual
    } else if capabilities.grammar {
        CodecId::Json
    } else {
        CodecId::Lenient
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codec_selection_follows_capabilities() {
        let native = CodecCapabilities {
            native_tools: true,
            xml_dialect: true,
            grammar: true,
        };
        assert_eq!(select_codec(native, None), CodecId::Native);

        let xml = CodecCapabilities {
            native_tools: false,
            xml_dialect: true,
            grammar: true,
        };
        assert_eq!(select_codec(xml, None), CodecId::XmlManual);

        let json = CodecCapabilities {
            native_tools: false,
            xml_dialect: false,
            grammar: true,
        };
        assert_eq!(select_codec(json, None), CodecId::Json);

        let lenient = CodecCapabilities {
            native_tools: false,
            xml_dialect: false,
            grammar: false,
        };
        assert_eq!(select_codec(lenient, None), CodecId::Lenient);
        assert_eq!(
            select_codec(native, Some(CodecId::Lenient)),
            CodecId::Lenient
        );
    }
}
