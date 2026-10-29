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
        evolution_key: None,
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

#[test]
fn an_existing_corridor_is_wired_through_the_port() {
    let dir = tempfile::tempdir().expect("tempdir");
    let providers = providers_dir("json");

    // No corridor: evolution is off by default (ADR-0015); nothing is wired.
    let runtime =
        build_runtime(config(&dir, providers.path().to_path_buf(), "known")).expect("build");
    assert!(runtime.evolution.is_none());

    // A corridor without a key is a visible warning, not a silent absence.
    std::fs::create_dir_all(dir.path().join(".orexis/evolution")).expect("corridor dir");
    let mut wired = config(&dir, providers.path().to_path_buf(), "known");
    wired.evolution_key = None;
    let runtime = build_runtime(wired).expect("build");
    assert!(runtime.evolution.is_none());
    assert!(runtime
        .warnings
        .iter()
        .any(|warning| warning.contains("OREXIS_EVOLUTION_KEY")));

    // A corridor with a key is wired as the EvolutionStore port.
    let mut wired = config(&dir, providers.path().to_path_buf(), "known");
    wired.evolution_key = Some("07".repeat(32));
    let runtime = build_runtime(wired).expect("build");
    let store = runtime.evolution.expect("corridor wired");
    let entries = futures::executor::block_on(store.entries()).expect("entries");
    assert!(entries.is_empty(), "a fresh corridor has no entries");
}

#[test]
fn auth_kind_visibility_matches_what_composition_can_build() {
    use orexis_adapters_provider_catalog::AuthKind;
    use orexis_composition::auth_kind_implemented;

    // Key and keyless auth build today; OAuth device flow, ADC, and
    // SigV4 are on-demand adapter work (ADR-0016) and must be visible as
    // unimplemented before selection.
    assert!(auth_kind_implemented(AuthKind::ApiKey));
    assert!(auth_kind_implemented(AuthKind::None));
    assert!(!auth_kind_implemented(AuthKind::OAuth));
    assert!(!auth_kind_implemented(AuthKind::Adc));
    assert!(!auth_kind_implemented(AuthKind::SigV4));
}

#[test]
fn selection_round_trips_through_the_project_config() {
    let dir = tempfile::tempdir().expect("tempdir");
    assert!(orexis_composition::read_selection(dir.path()).is_none());
    orexis_composition::write_selection(
        dir.path(),
        &orexis_composition::Selection {
            profile_id: "openai".to_string(),
            model_id: "gpt-5".to_string(),
        },
    )
    .expect("write");
    let selection = orexis_composition::read_selection(dir.path()).expect("selection");
    assert_eq!(selection.profile_id, "openai");
    assert_eq!(selection.model_id, "gpt-5");
}

#[test]
fn load_catalog_finds_the_vendored_providers() {
    let catalog = orexis_composition::load_catalog().expect("catalog");
    let ids: Vec<&str> = catalog.list().map(|profile| profile.id.as_str()).collect();
    assert!(
        ids.contains(&"openai") && ids.contains(&"anthropic"),
        "the shipped profiles are listed: {ids:?}"
    );
}

#[test]
fn unimplemented_auth_fails_at_composition_naming_it() {
    // A profile whose auth kind is not implemented must fail at
    // composition with an error that says so — never at the first turn
    // (ticket 014, ADR-0016).
    let dir = tempfile::tempdir().expect("tempdir");
    let providers = tempfile::tempdir().expect("providers");
    std::fs::write(
        providers.path().join("oauth.yaml"),
        r#"
id: oauth-test
display_name: OAuth Test
kind: openai_compatible
base_url: https://127.0.0.1:1/v1
auth:
  kind: o_auth
wire:
  dialect: open_ai
  tool_format: native
  stream_format: sse
capabilities:
  tools: true
models:
  - id: known
    context: 32768
limits:
  max_concurrency: 1
"#,
    )
    .expect("write profile");
    let error = match build_runtime(BuildConfig {
        profile_id: "oauth-test".to_string(),
        ..config(&dir, providers.path().to_path_buf(), "known")
    }) {
        Err(error) => error,
        Ok(_) => panic!("expected the build to be refused"),
    };
    match error {
        orexis_composition::BuildError::Unsupported(message) => {
            assert!(message.contains("oauth"), "names the auth kind: {message}");
            assert!(
                message.contains("not implemented"),
                "says it is not implemented: {message}"
            );
        }
        other => panic!("expected Unsupported, got {other:?}"),
    }
}

#[test]
fn explicit_providers_dir_overrides_shipped_while_preserving_others() {
    let dir = tempfile::tempdir().expect("tempdir");
    let custom = tempfile::tempdir().expect("custom");
    std::fs::write(
        custom.path().join("llamacpp.yaml"),
        r#"
id: llamacpp
display_name: llama.cpp custom
kind: llamacpp
base_url: http://127.0.0.1:8080/v1
wire:
  dialect: open_ai
  tool_format: native
  stream_format: sse
capabilities:
  tools: true
models:
  - id: local
    context: 32768
limits:
  max_concurrency: 2
"#,
    )
    .expect("write profile");

    let runtime = build_runtime(BuildConfig {
        profile_id: "llamacpp".to_string(),
        model_id: "local".to_string(),
        db_path: dir.path().join("s.db"),
        project_root: dir.path().to_path_buf(),
        cwd: dir.path().to_path_buf(),
        providers_dir: Some(custom.path().to_path_buf()),
        mcp_dirs: Some(Vec::new()),
        utcp_dirs: Some(Vec::new()),
        a2a_dirs: Some(Vec::new()),
        evolution_key: None,
        verify_command: None,
        system_prompt: String::new(),
        context_limit: 32768,
    })
    .expect("build llamacpp");
    assert_eq!(runtime.generation_slots, 2);

    let runtime_openai = build_runtime(BuildConfig {
        profile_id: "openai".to_string(),
        model_id: "gpt-4o".to_string(),
        db_path: dir.path().join("s2.db"),
        project_root: dir.path().to_path_buf(),
        cwd: dir.path().to_path_buf(),
        providers_dir: Some(custom.path().to_path_buf()),
        mcp_dirs: Some(Vec::new()),
        utcp_dirs: Some(Vec::new()),
        a2a_dirs: Some(Vec::new()),
        evolution_key: None,
        verify_command: None,
        system_prompt: String::new(),
        context_limit: 32768,
    })
    .expect("build openai");
    assert_eq!(runtime_openai.codec.name(), "native");
}

#[test]
fn export_bundle_and_import_bundle_round_trip_sessions_and_entity() {
    use futures::executor::block_on;
    use orexis_core::{Drive, EntityEvent, SessionEvent};

    let src_dir = tempfile::tempdir().expect("src");
    let dst_dir = tempfile::tempdir().expect("dst");
    let providers = providers_dir("native");

    let src_db = src_dir.path().join("s.db");
    let rt = build_runtime(config(&src_dir, providers.path().to_path_buf(), "known"))
        .expect("build src");

    let s1 = rt.sessions.open(None).expect("open s1");
    block_on(s1.append.append(SessionEvent::UserMessage {
        text: "hello export".to_string(),
    }))
    .expect("append user");

    let entity = rt.entity.open().expect("open entity");
    block_on(entity.append.append(EntityEvent::DriveUpdate {
        drive: Drive::Curiosity,
        value: 0.9,
        reason: "explore".to_string(),
    }))
    .expect("append entity");

    let bundle = orexis_composition::export_bundle(&src_db, None).expect("export bundle");
    let dst_db = dst_dir.path().join("s.db");
    let imported = orexis_composition::import_bundle(&dst_db, &bundle).expect("import bundle");
    assert!(imported >= 2, "expected session + entity records imported");

    let dst_rt = build_runtime(config(&dst_dir, providers.path().to_path_buf(), "known"))
        .expect("build dst");
    let dst_s1 = dst_rt.sessions.open(Some(&s1.id)).expect("open dst s1");
    assert_eq!(
        block_on(s1.read.events()).expect("src events"),
        block_on(dst_s1.read.events()).expect("dst events")
    );
    let dst_entity = dst_rt.entity.open().expect("open dst entity");
    let snap = block_on(dst_entity.read.snapshot()).expect("dst snapshot");
    assert_eq!(snap.drives.curiosity, 0.9);
}
