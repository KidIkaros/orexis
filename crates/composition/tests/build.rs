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
fn unknown_models_warn_and_keep_capacity() {
    let dir = tempfile::tempdir().expect("tempdir");
    let providers = providers_dir("native");
    let runtime =
        build_runtime(config(&dir, providers.path().to_path_buf(), "unknown")).expect("build");
    assert_eq!(runtime.generation_slots, 3);
    assert!(!runtime.warnings.is_empty());
}
