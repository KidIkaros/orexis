//! Integration: the stdio LSP client against a mock server.

use std::time::Duration;

use orexis_adapters_lsp::StdioLsp;
use orexis_ports::Lsp;

#[tokio::test]
async fn mock_server_reports_a_diagnostic() {
    let dir = tempfile::tempdir().expect("tempdir");
    let lsp = StdioLsp::new(dir.path(), env!("CARGO_BIN_EXE_mock_lsp"))
        .with_timeout(Duration::from_secs(3));
    let diagnostics = lsp
        .diagnostics("src/main.rs", "fn main() {}")
        .await
        .expect("diagnostics");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].severity, "error");
    assert!(diagnostics[0].message.contains("mock"));
    assert!(diagnostics[0].path.ends_with("src/main.rs"));
}
