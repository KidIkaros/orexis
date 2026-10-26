use super::*;

#[test]
fn provider_type_names() {
    assert_eq!(ProviderType::Cli.name(), "cli");
    assert_eq!(ProviderType::Http.name(), "http");
    assert_eq!(ProviderType::Mcp.name(), "mcp");
}

#[test]
fn parse_providers_accepts_both_shapes() {
    let array = parse_providers(r#"[{"name":"a","provider_type":"http","endpoint":"https://a"}]"#)
        .expect("array");
    assert_eq!(array.len(), 1);

    let object =
        parse_providers(r#"{"providers": [{"name":"b","provider_type":"cli","command":["sh"]}]}"#)
            .expect("object");
    assert_eq!(object.len(), 1);
    assert_eq!(object[0].provider_type, ProviderType::Cli);
}

#[test]
fn parse_providers_rejects_malformed() {
    assert!(matches!(
        parse_providers("not json"),
        Err(UtcpError::Parse(_))
    ));
    assert!(matches!(
        parse_providers(r#"{"foo":"bar"}"#),
        Err(UtcpError::Shape)
    ));
    assert!(parse_providers(r#"[{"name":"a","provider_type":"http"}]"#).is_err());
}

#[test]
fn validate_rejects_unreachable_providers() {
    let cli = Provider {
        name: "c".into(),
        provider_type: ProviderType::Cli,
        endpoint: String::new(),
        command: Vec::new(),
        command_args: Vec::new(),
        auth_token: None,
        description: String::new(),
    };
    assert!(cli.validate().is_err());
    let mcp = Provider {
        provider_type: ProviderType::Mcp,
        ..cli
    };
    assert!(mcp.validate().is_err());
}
