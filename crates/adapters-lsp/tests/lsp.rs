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

#[tokio::test]
async fn a_second_edit_reuses_the_server() {
    let dir = tempfile::tempdir().expect("tempdir");
    let lsp = StdioLsp::new(dir.path(), env!("CARGO_BIN_EXE_mock_lsp"))
        .with_timeout(Duration::from_secs(3));
    let first = lsp
        .diagnostics("src/main.rs", "fn main() {}")
        .await
        .expect("first");
    let second = lsp
        .diagnostics("src/main.rs", "fn main() { let x = 1; }")
        .await
        .expect("second");
    assert!(!first.is_empty());
    assert!(!second.is_empty());
    assert_eq!(lsp.spawns(), 1);
}
