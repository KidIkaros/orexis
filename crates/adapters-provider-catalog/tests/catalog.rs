//! Catalog, precedence, and secret tests.

use std::collections::BTreeMap;

use orexis_adapters_provider_catalog::*;

const OPENAI: &str = r#"
id: openai
display_name: OpenAI
kind: openai_compatible
base_url: https://api.openai.com/v1
auth:
  env: [OPENAI_API_KEY]
models:
  - id: gpt-5
    context: 400000
"#;

#[test]
fn parses_a_profile() {
    let profile: ProviderProfile = serde_yaml::from_str(OPENAI).unwrap();
    assert_eq!(profile.id, "openai");
    assert_eq!(profile.auth.header, "authorization");
    assert_eq!(profile.limits.max_concurrency, 4);
    assert_eq!(profile.models[0].context, 400000);
}

#[test]
fn later_directories_win() {
    let shipped = tempfile::tempdir().unwrap();
    let user = tempfile::tempdir().unwrap();
    std::fs::write(shipped.path().join("openai.yaml"), OPENAI).unwrap();
    let override_yaml = OPENAI.replace("gpt-5", "gpt-5-mini");
    std::fs::write(user.path().join("openai.yaml"), override_yaml).unwrap();

    let catalog =
        Catalog::load(&[shipped.path().to_path_buf(), user.path().to_path_buf()]).unwrap();
    let profile = catalog.get("openai").unwrap();
    assert_eq!(profile.models[0].id, "gpt-5-mini");
    assert!(catalog
        .source_of("openai")
        .unwrap()
        .starts_with(user.path()));
}

#[test]
fn env_secret_resolves() {
    std::env::set_var("OREXIS_TEST_KEY_123", "secret-value");
    let profile: ProviderProfile =
        serde_yaml::from_str(&OPENAI.replace("OPENAI_API_KEY", "OREXIS_TEST_KEY_123")).unwrap();
    assert_eq!(EnvSecrets.get(&profile), Some("secret-value".to_string()));
    std::env::remove_var("OREXIS_TEST_KEY_123");
}

#[test]
fn memory_secret_resolves() {
    let profile: ProviderProfile = serde_yaml::from_str(OPENAI).unwrap();
    let mut values = BTreeMap::new();
    values.insert("openai".to_string(), "mem".to_string());
    let store = MemorySecrets::new(values);
    assert_eq!(store.get(&profile), Some("mem".to_string()));
}

#[test]
fn unknown_model_gets_conservative_defaults() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("openai.yaml"), OPENAI).unwrap();
    let catalog = Catalog::load(&[dir.path().to_path_buf()]).unwrap();
    let resolution = catalog.resolve_model("openai", "not-a-model").unwrap();
    assert!(!resolution.known);
    assert_eq!(resolution.model.context, 32768);
    assert!(resolution.warning.is_some());
}

#[derive(Default)]
struct MemoryKeyring {
    values: std::sync::Mutex<BTreeMap<(String, String), String>>,
}

impl KeyringBackend for MemoryKeyring {
    fn get(&self, service: &str, account: &str) -> Result<Option<String>, String> {
        Ok(self
            .values
            .lock()
            .expect("values")
            .get(&(service.to_string(), account.to_string()))
            .cloned())
    }

    fn set(&self, service: &str, account: &str, secret: &str) -> Result<(), String> {
        self.values.lock().expect("values").insert(
            (service.to_string(), account.to_string()),
            secret.to_string(),
        );
        Ok(())
    }

    fn delete(&self, service: &str, account: &str) -> Result<(), String> {
        self.values
            .lock()
            .expect("values")
            .remove(&(service.to_string(), account.to_string()));
        Ok(())
    }
}

#[test]
fn keyring_store_and_forget() {
    let store = KeyringSecrets::with_backend("dev.orexis", MemoryKeyring::default());
    store.store("openai", "sk-test").expect("store");
    let profile: ProviderProfile = serde_yaml::from_str(OPENAI).expect("profile");
    assert_eq!(store.get(&profile), Some("sk-test".to_string()));
    store.forget("openai").expect("forget");
    assert_eq!(store.get(&profile), None);
}
