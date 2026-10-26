use super::*;
use crate::manual::ProviderType;

#[test]
fn provider_type_names_and_deserialize() {
    assert_eq!(ProviderType::Cli.name(), "cli");
    assert_eq!(ProviderType::Http.name(), "http");
    assert_eq!(ProviderType::Mcp.name(), "mcp");
    for (wire, expected) in [
        ("\"cli\"", ProviderType::Cli),
        ("\"http\"", ProviderType::Http),
        ("\"mcp\"", ProviderType::Mcp),
    ] {
        let parsed: ProviderType = serde_json::from_str(wire).expect("parse");
        assert_eq!(parsed, expected);
    }
}

#[test]
fn provider_deserializes_minimally() {
    let provider: Provider = serde_json::from_str(
        r#"{"name":"github","provider_type":"http","endpoint":"https://api.github.com"}"#,
    )
    .expect("parse");
    assert_eq!(provider.name, "github");
    assert_eq!(provider.provider_type, ProviderType::Http);
    assert_eq!(provider.endpoint, "https://api.github.com");
    assert!(provider.auth_token.is_none());
}

#[test]
fn load_providers_accepts_array_and_object_forms() {
    let mut core = UtcpCore::new();
    let count = core
        .load_providers(
            r#"[{"name":"a","provider_type":"http","endpoint":"https://a"},{"name":"b","provider_type":"cli","command":["sh"]}]"#,
        )
        .expect("array");
    assert_eq!(count, 2);
    assert_eq!(core.provider_count(), 2);

    let count = core
        .load_providers(
            r#"{"providers": [{"name":"c","provider_type":"http","endpoint":"https://c"}]}"#,
        )
        .expect("object");
    assert_eq!(count, 1);
    assert_eq!(core.provider_count(), 3);
}

#[test]
fn load_providers_rejects_malformed_and_invalid() {
    let mut core = UtcpCore::new();
    assert!(core.load_providers("not json").is_err());
    assert!(core.load_providers(r#"{"foo":"bar"}"#).is_err());
    // Structural validation: an http provider without an endpoint.
    assert!(core
        .load_providers(r#"[{"name":"x","provider_type":"http"}]"#)
        .is_err());
    // A name containing '/' would break the tool namespace.
    assert!(core
        .load_providers(r#"[{"name":"a/b","provider_type":"http","endpoint":"https://a"}]"#)
        .is_err());
}

#[test]
fn provider_names_are_sorted() {
    let mut core = UtcpCore::new();
    core.load_providers(
        r#"[{"name":"zeta","provider_type":"http","endpoint":"https://z"},{"name":"alpha","provider_type":"http","endpoint":"https://a"}]"#,
    )
    .expect("load");
    assert_eq!(core.provider_names(), vec!["alpha", "zeta"]);
}

#[test]
fn http_discover_instruction_builds_the_url_and_bearer() {
    let mut core = UtcpCore::new();
    core.load_providers(
        r#"[{"name":"github","provider_type":"http","endpoint":"https://api.github.com/","auth_token":"t0k3n"}]"#,
    )
    .expect("load");
    let instructions = core.discover_instructions();
    assert_eq!(
        instructions,
        vec![(
            "github".to_string(),
            DiscoverInstruction::Http {
                url: "https://api.github.com/tools".to_string(),
                bearer: Some("t0k3n".to_string()),
            }
        )]
    );
}

#[test]
fn cli_discover_instruction_appends_list_tools() {
    let mut core = UtcpCore::new();
    core.load_providers(
        r#"[{"name":"mock","provider_type":"cli","command":["sh","-c","echo tools"]}]"#,
    )
    .expect("load");
    let instructions = core.discover_instructions();
    assert_eq!(
        instructions,
        vec![(
            "mock".to_string(),
            DiscoverInstruction::Cli {
                program: "sh".to_string(),
                args: vec![
                    "-c".to_string(),
                    "echo tools".to_string(),
                    "--list-tools".to_string()
                ],
            }
        )]
    );
}

#[test]
fn mcp_discover_instruction_uses_command_args_or_endpoint() {
    let mut core = UtcpCore::new();
    core.load_providers(
        r#"[{"name":"fs","provider_type":"mcp","command_args":["npx","-y","server"]},{"name":"bare","provider_type":"mcp","endpoint":"/usr/bin/server"}]"#,
    )
    .expect("load");
    let instructions = core.discover_instructions();
    assert_eq!(
        instructions[0].1,
        DiscoverInstruction::Mcp {
            command: vec!["npx".into(), "-y".into(), "server".into()],
        }
    );
    assert_eq!(
        instructions[1].1,
        DiscoverInstruction::Mcp {
            command: vec!["/usr/bin/server".into()],
        }
    );
}

#[test]
fn record_discovery_replaces_the_provider_entries() {
    let mut core = UtcpCore::new();
    core.record_discovery(
        "github",
        vec![ToolSchema {
            name: "list_issues".into(),
            description: "list issues".into(),
            input_schema: serde_json::json!({"type": "object"}),
            provider: "github".into(),
        }],
    );
    core.record_discovery(
        "github",
        vec![ToolSchema {
            name: "create_issue".into(),
            description: String::new(),
            input_schema: serde_json::json!({"type": "object"}),
            provider: String::new(),
        }],
    );
    assert_eq!(core.tool_count(), 1);
    assert_eq!(core.tools()[0].name, "create_issue");
    assert_eq!(core.tools()[0].provider, "github");
}

#[test]
fn search_ranks_name_over_description_case_insensitively() {
    let mut core = UtcpCore::new();
    core.record_discovery(
        "p",
        vec![
            ToolSchema {
                name: "other".into(),
                description: "searches the web".into(),
                input_schema: serde_json::json!({}),
                provider: "p".into(),
            },
            ToolSchema {
                name: "web_search".into(),
                description: String::new(),
                input_schema: serde_json::json!({}),
                provider: "p".into(),
            },
        ],
    );
    let results = core.search_tools("SEARCH");
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].name, "web_search");
    assert_eq!(results[1].name, "other");
}

#[test]
fn find_tool_matches_exactly() {
    let mut core = UtcpCore::new();
    core.record_discovery(
        "p",
        vec![ToolSchema {
            name: "echo".into(),
            description: String::new(),
            input_schema: serde_json::json!({}),
            provider: "p".into(),
        }],
    );
    assert!(core.find_tool("echo").is_some());
    assert!(core.find_tool("ech").is_none());
    assert!(core.find_tool("echoes").is_none());
}

#[test]
fn http_call_instruction_substitutes_path_body_and_auth() {
    let mut core = UtcpCore::new();
    core.load_providers(
        r#"[{"name":"github","provider_type":"http","endpoint":"https://api.github.com","auth_token":"t0k3n"}]"#,
    )
    .expect("load");
    core.record_discovery(
        "github",
        vec![ToolSchema {
            name: "list_issues".into(),
            description: String::new(),
            input_schema: serde_json::json!({}),
            provider: "github".into(),
        }],
    );
    let (provider, instruction) = core
        .call_instruction("list_issues", serde_json::json!({"repo": "x/y"}))
        .expect("instruction");
    assert_eq!(provider, "github");
    assert_eq!(
        instruction,
        CallInstruction::Http {
            url: "https://api.github.com/tools/list_issues".to_string(),
            bearer: Some("t0k3n".to_string()),
            body: serde_json::json!({"repo": "x/y"}),
        }
    );
}

#[test]
fn cli_call_instruction_appends_tool_and_args_json() {
    let mut core = UtcpCore::new();
    core.load_providers(r#"[{"name":"mock","provider_type":"cli","command":["sh","-c","run"]}]"#)
        .expect("load");
    core.record_discovery(
        "mock",
        vec![ToolSchema {
            name: "echo".into(),
            description: String::new(),
            input_schema: serde_json::json!({}),
            provider: "mock".into(),
        }],
    );
    let (_, instruction) = core
        .call_instruction("echo", serde_json::json!({"text": "hi"}))
        .expect("instruction");
    assert_eq!(
        instruction,
        CallInstruction::Cli {
            program: "sh".to_string(),
            args: vec![
                "-c".to_string(),
                "run".to_string(),
                "echo".to_string(),
                r#"{"text":"hi"}"#.to_string(),
            ],
        }
    );
}

#[test]
fn mcp_call_instruction_carries_the_tool_and_args() {
    let mut core = UtcpCore::new();
    core.load_providers(r#"[{"name":"fs","provider_type":"mcp","command_args":["npx","server"]}]"#)
        .expect("load");
    core.record_discovery(
        "fs",
        vec![ToolSchema {
            name: "read_file".into(),
            description: String::new(),
            input_schema: serde_json::json!({}),
            provider: "fs".into(),
        }],
    );
    let (_, instruction) = core
        .call_instruction("read_file", serde_json::json!({"path": "a"}))
        .expect("instruction");
    assert_eq!(
        instruction,
        CallInstruction::Mcp {
            command: vec!["npx".into(), "server".into()],
            tool: "read_file".into(),
            args: serde_json::json!({"path": "a"}),
        }
    );
}

#[test]
fn unknown_tools_and_providers_error() {
    let mut core = UtcpCore::new();
    core.load_providers(
        r#"[{"name":"github","provider_type":"http","endpoint":"https://api.github.com"}]"#,
    )
    .expect("load");
    let error = core
        .call_instruction("missing", serde_json::json!({}))
        .expect_err("unknown tool");
    assert!(matches!(error, UtcpError::UnknownTool(_)));
}
