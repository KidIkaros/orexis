//! Integration: the real composition path selects the right codec, carries
//! provider capacity, and surfaces unknown-model warnings.

use std::path::PathBuf;

use orexis_composition::{build_runtime, BuildConfig};

fn providers_dir(tool_format: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    let yaml = format!(
        r#"
id: test
display_name: Test
kind: openai_compatible
base_url: http://127.0.0.1:1/v1
wire:
  dialect: open_ai
  tool_format: {tool_format}
  stream_format: sse
capabilities:
  tools: true
  json_mode: false
models:
  - id: known
    context: 32768
limits:
  max_concurrency: 3
"#
    );
    std::fs::write(dir.path().join("test.yaml"), yaml).expect("write");
    dir
}

fn config(dir: &tempfile::TempDir, providers: PathBuf, model: &str) -> BuildConfig {
    BuildConfig {
        profile_id: "test".to_string(),
        model_id: model.to_string(),
        db_path: dir.path().join("s.db"),
        project_root: dir.path().to_path_buf(),
        cwd: dir.path().to_path_buf(),
        providers_dir: Some(providers),
        mcp_dirs: Some(Vec::new()),
        utcp_dirs: Some(Vec::new()),
        a2a_dirs: Some(Vec::new()),
        verify_command: None,
        system_prompt: String::new(),
        context_limit: 32768,
    }
}

#[test]
fn xml_profiles_select_xml_manual() {
    let dir = tempfile::tempdir().expect("tempdir");
    let providers = providers_dir("xml");
    let runtime =
        build_runtime(config(&dir, providers.path().to_path_buf(), "known")).expect("build");
    assert_eq!(runtime.codec.name(), "xml-manual");
    assert_eq!(runtime.generation_slots, 3);
    assert!(runtime
        .context_tools
        .iter()
        .any(|tool| tool.name == "read_file"));
    assert!(runtime.warnings.is_empty());
}

#[test]
fn native_profiles_select_native() {
    let dir = tempfile::tempdir().expect("tempdir");
    let providers = providers_dir("native");
    let runtime =
        build_runtime(config(&dir, providers.path().to_path_buf(), "known")).expect("build");
    assert_eq!(runtime.codec.name(), "native");
}

#[test]
fn native_gemini_wire_is_on_demand_and_names_the_compatible_path() {
    let dir = tempfile::tempdir().expect("tempdir");
    let yaml = r#"
id: gemini-native
display_name: Gemini (native)
kind: gemini
base_url: https://generativelanguage.googleapis.com/v1beta
auth:
  kind: api_key
  env: [GEMINI_API_KEY]
wire:
  dialect: gemini
  tool_format: native
  stream_format: sse
capabilities:
  tools: true
models:
  - id: gemini-2.5-flash
    context: 1048576
limits:
  max_concurrency: 4
"#;
    std::fs::write(dir.path().join("gemini-native.yaml"), yaml).expect("write");
    let build_config = BuildConfig {
        profile_id: "gemini-native".to_string(),
        ..config(&dir, dir.path().to_path_buf(), "gemini-2.5-flash")
    };
    let err = match build_runtime(build_config) {
        Err(err) => err,
        Ok(_) => panic!("the native Gemini wire is on demand"),
    };
    let message = err.to_string();
    assert!(message.contains("on demand"), "message: {message}");
    assert!(message.contains("openai_compatible"), "message: {message}");
}

#[test]
fn mcp_servers_load_from_explicit_dirs() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mcp = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        mcp.path().join("servers.json"),
        r#"{"servers": [{"name": "fs", "command": "npx", "args": ["-y", "server"]}]}"#,
    )
    .expect("write");
    let providers = providers_dir("native");
    let build_config = BuildConfig {
        mcp_dirs: Some(vec![mcp.path().to_path_buf()]),
        ..config(&dir, providers.path().to_path_buf(), "known")
    };
    let runtime = build_runtime(build_config).expect("build");
    assert!(runtime.external.iter().any(|source| source.kind() == "mcp"));
}

#[test]
fn no_external_configs_means_no_external_layer() {
    let dir = tempfile::tempdir().expect("tempdir");
    let providers = providers_dir("native");
    let runtime =
        build_runtime(config(&dir, providers.path().to_path_buf(), "known")).expect("build");
    assert!(runtime.external.is_empty());
}

#[test]
fn unknown_models_warn_and_keep_capacity() {
    let dir = tempfile::tempdir().expect("tempdir");
    let providers = providers_dir("native");
    let runtime =
        build_runtime(config(&dir, providers.path().to_path_buf(), "unknown")).expect("build");
    assert_eq!(runtime.generation_slots, 3);
    assert!(!runtime.warnings.is_empty());
}

#[test]
fn utcp_manuals_load_from_explicit_dirs() {
    let dir = tempfile::tempdir().expect("tempdir");
    let utcp = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        utcp.path().join("providers.json"),
        r#"[{"name":"svc","provider_type":"http","endpoint":"https://svc.example.com"}]"#,
    )
    .expect("write");
    let providers = providers_dir("native");
    let build_config = BuildConfig {
        utcp_dirs: Some(vec![utcp.path().to_path_buf()]),
        ..config(&dir, providers.path().to_path_buf(), "known")
    };
    let runtime = build_runtime(build_config).expect("build");
    assert!(runtime
        .external
        .iter()
        .any(|source| source.kind() == "utcp"));
}
