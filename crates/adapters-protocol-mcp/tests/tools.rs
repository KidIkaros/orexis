//! Conformance fixtures for `McpTools` against the scripted mock server.

use orexis_adapters_protocol_mcp::{load_server_configs, McpServerConfig, McpTools};
use orexis_ports::ToolExecutor;
use std::path::PathBuf;

fn mock_config(name: &str, args: &[&str]) -> McpServerConfig {
    McpServerConfig {
        name: name.into(),
        command: env!("CARGO_BIN_EXE_mock_mcp_server").into(),
        args: args.iter().map(|arg| arg.to_string()).collect(),
        env: Default::default(),
    }
}

#[tokio::test]
async fn connect_all_reports_failures_and_specs_collect() {
    let tools = McpTools::new(vec![
        mock_config("mock", &[]),
        McpServerConfig {
            name: "missing".into(),
            command: "/nonexistent/mcp-server-xyz".into(),
            args: Vec::new(),
            env: Default::default(),
        },
    ]);
    let failures = tools.connect_all().await;
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].0, "missing");
    let specs = tools.specs().await;
    assert_eq!(specs.len(), 1);
    assert_eq!(specs[0].name, "mcp/mock/mock_tool");
    assert_eq!(specs[0].description, "a mock tool");
}

#[tokio::test]
async fn execute_routes_and_delimits_output() {
    let tools = McpTools::new(vec![mock_config("mock", &[])]);
    tools.connect_all().await;
    let observation = tools
        .execute("mcp/mock/mock_tool", r#"{"path": "a.rs"}"#)
        .await
        .expect("execute");
    assert_eq!(observation.name, "mcp/mock/mock_tool");
    assert!(observation
        .output
        .starts_with("<mcp_result server=\"mock\" tool=\"mock_tool\">"));
    assert!(observation.output.contains("mock says hi"));
    assert!(observation.output.trim_end().ends_with("</mcp_result>"));
}

#[tokio::test]
async fn execute_rejects_bad_names_and_args() {
    let tools = McpTools::new(vec![mock_config("mock", &[])]);
    tools.connect_all().await;
    assert!(tools.execute("read_file", "{}").await.is_err());
    assert!(tools.execute("mcp/mock/", "{}").await.is_err());
    assert!(tools.execute("mcp/other/tool", "{}").await.is_err());
    assert!(tools
        .execute("mcp/mock/mock_tool", "not json")
        .await
        .is_err());
}

#[tokio::test]
async fn execute_escapes_delimiter_breakout_in_output() {
    let tools = McpTools::new(vec![mock_config("mock", &["inject"])]);
    tools.connect_all().await;
    let observation = tools
        .execute("mcp/mock/mock_tool", "")
        .await
        .expect("execute");
    assert_eq!(observation.output.matches("</mcp_result>").count(), 1);
    assert!(observation.output.contains("<\\/mcp_result"));
}

#[tokio::test]
async fn execute_caps_huge_output() {
    let tools = McpTools::new(vec![mock_config("mock", &["huge"])]);
    tools.connect_all().await;
    let observation = tools
        .execute("mcp/mock/mock_tool", "")
        .await
        .expect("execute");
    assert!(observation.output.contains("[truncated at 64KB]"));
    assert!(observation.output.len() < 64 * 1024 + 512);
}

#[test]
fn load_server_configs_reads_the_project_dir() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("servers.json"),
        r#"{"servers": [{"name": "fs", "command": "npx", "args": ["-y", "server"], "env": {"K": "v"}}]}"#,
    )
    .expect("write");
    let (configs, problems) = load_server_configs(&[PathBuf::from(dir.path())]);
    assert!(problems.is_empty());
    assert_eq!(configs.len(), 1);
    assert_eq!(configs[0].name, "fs");
    assert_eq!(configs[0].args, vec!["-y", "server"]);
    assert_eq!(configs[0].env.get("K"), Some(&"v".to_string()));
}
