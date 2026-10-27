//! The `providers` and `init` surfaces (ticket 014): a profile's auth
//! kind and credential are visible *before* selection, so the user never
//! picks a profile that cannot build (ADR-0016, the 1.0 checklist's "no
//! silent default in disguise").

use std::path::Path;

use orexis_adapters_provider_catalog::{Catalog, EnvSecrets, KeyringSecrets, SecretStore};
use orexis_composition::{auth_kind_implemented, load_catalog};

/// One profile row of the listing.
pub struct ProfileRow {
    pub id: String,
    pub display_name: String,
    pub kind: String,
    pub auth_kind: String,
    pub auth_implemented: bool,
    pub credential: bool,
    pub models: usize,
}

/// Every profile with its buildability visible.
pub fn profile_rows(catalog: &Catalog) -> Vec<ProfileRow> {
    let keyring = KeyringSecrets::new("dev.orexis");
    catalog
        .list()
        .map(|profile| {
            let credential = keyring.get(profile).is_some() || EnvSecrets.get(profile).is_some();
            ProfileRow {
                id: profile.id.clone(),
                display_name: profile.display_name.clone(),
                kind: format!("{:?}", profile.kind),
                auth_kind: format!("{:?}", profile.auth.kind),
                auth_implemented: auth_kind_implemented(profile.auth.kind),
                credential,
                models: profile.models.len(),
            }
        })
        .collect()
}

/// The `providers list` rendering.
pub fn list(catalog: &Catalog) -> String {
    let mut out = String::new();
    for row in profile_rows(catalog) {
        let auth = if row.auth_implemented {
            format!("auth: {}", row.auth_kind)
        } else {
            format!("auth: {} (not implemented yet)", row.auth_kind)
        };
        let credential = if row.credential {
            "credential set"
        } else {
            "no credential"
        };
        out.push_str(&format!(
            "{}  {}  [{} · {} · {} models]\n",
            row.id, row.display_name, row.kind, auth, row.models
        ));
        if !row.auth_implemented || !row.credential {
            out.push_str(&format!("    {credential}\n"));
        }
    }
    out
}

/// The `providers show <id>` rendering.
pub fn show(catalog: &Catalog, id: &str) -> Option<String> {
    let profile = catalog.get(id)?;
    let row = profile_rows(catalog)
        .into_iter()
        .find(|row| row.id == id)
        .expect("the profile was just listed");
    let models = profile
        .models
        .iter()
        .map(|model| format!("    {} (context {})\n", model.id, model.context))
        .collect::<Vec<_>>()
        .join("");
    Some(format!(
        "{} — {}\nkind: {}\nauth: {}{}\nwire dialect: {:?}\nsource: {}\nmodels:\n{}",
        profile.id,
        profile.display_name,
        row.kind,
        row.auth_kind,
        if row.auth_implemented {
            String::new()
        } else {
            " (not implemented yet — this profile cannot build; see ADR-0016)".to_string()
        },
        profile.wire.dialect,
        catalog
            .source_of(id)
            .map(|path| path.display().to_string())
            .unwrap_or_default(),
        if models.is_empty() {
            "    (none declared)\n".to_string()
        } else {
            models
        },
    ))
}

/// Why an `init` selection was refused.
#[derive(Debug, thiserror::Error)]
pub enum InitError {
    #[error("unknown profile: {0}")]
    UnknownProfile(String),
    #[error("profile {0} uses {1} auth, which is not implemented yet; pick another (ADR-0016)")]
    UnimplementedAuth(String, String),
    #[error("catalog: {0}")]
    Catalog(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

/// Validate a selection and write it. Unknown models are allowed with a
/// warning (the catalog synthesizes conservative defaults); an unknown
/// profile or an unimplemented auth kind is refused — that is the
/// "cannot build" case this ticket exists to prevent.
pub fn init_selection(
    project_root: &Path,
    profile_id: &str,
    model_id: &str,
) -> Result<String, InitError> {
    let catalog = load_catalog().map_err(|error| InitError::Catalog(error.to_string()))?;
    let Some(profile) = catalog.get(profile_id) else {
        return Err(InitError::UnknownProfile(profile_id.to_string()));
    };
    if !auth_kind_implemented(profile.auth.kind) {
        return Err(InitError::UnimplementedAuth(
            profile_id.to_string(),
            format!("{:?}", profile.auth.kind),
        ));
    }
    let known = profile.models.iter().any(|model| model.id == model_id);
    if !known {
        eprintln!(
            "warning: model {model_id} is not declared by {profile_id}; \
             conservative limits apply until the catalog knows it"
        );
    }
    let keyring = KeyringSecrets::new("dev.orexis");
    let credential = keyring.get(profile).is_some() || EnvSecrets.get(profile).is_some();
    if !credential {
        let env_names = profile.auth.env.join(", ");
        eprintln!(
            "warning: no credential for {profile_id} ({env_names} env or the keyring); \
             set one before the first turn"
        );
    }
    orexis_composition::write_selection(
        project_root,
        &orexis_composition::Selection {
            profile_id: profile_id.to_string(),
            model_id: model_id.to_string(),
        },
    )?;
    Ok(format!("selected {profile_id} / {model_id}\n"))
}

/// The interactive picker: numbered implemented profiles, then numbered
/// models. Unimplemented profiles are visible but not selectable.
pub fn init_interactive(project_root: &Path) -> Result<String, InitError> {
    let catalog = load_catalog().map_err(|error| InitError::Catalog(error.to_string()))?;
    let rows = profile_rows(&catalog);
    let buildable: Vec<&ProfileRow> = rows.iter().filter(|row| row.auth_implemented).collect();
    println!("providers (a profile marked \"not implemented yet\" cannot be selected):");
    for row in &rows {
        let mark = if row.auth_implemented {
            String::new()
        } else {
            "  (not implemented yet)".to_string()
        };
        println!("  {} — {}{}", row.id, row.display_name, mark);
    }
    print!(
        "pick a provider [{}]: ",
        buildable
            .iter()
            .map(|row| row.id.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
    use std::io::Write;
    std::io::stdout().flush().ok();
    let mut line = String::new();
    std::io::stdin().read_line(&mut line)?;
    let profile_id = line.trim().to_string();
    let Some(profile) = catalog.get(&profile_id) else {
        return Err(InitError::UnknownProfile(profile_id));
    };
    if !auth_kind_implemented(profile.auth.kind) {
        return Err(InitError::UnimplementedAuth(
            profile_id,
            format!("{:?}", profile.auth.kind),
        ));
    }
    println!("models of {profile_id}:");
    for model in &profile.models {
        println!("  {} (context {})", model.id, model.context);
    }
    print!("pick a model: ");
    std::io::stdout().flush().ok();
    line.clear();
    std::io::stdin().read_line(&mut line)?;
    let model_id = line.trim().to_string();
    init_selection(project_root, &profile_id, &model_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog() -> Catalog {
        load_catalog().expect("catalog")
    }

    #[test]
    fn the_listing_marks_unimplemented_profiles() {
        let rows = profile_rows(&catalog());
        let openai = rows.iter().find(|row| row.id == "openai").expect("openai");
        assert!(openai.auth_implemented, "api_key auth builds today");
        // The hermes snapshot ships profiles whose auth is not wired
        // (OAuth device flow, SigV4, ADC); the listing must say so.
        let unimplemented: Vec<&ProfileRow> =
            rows.iter().filter(|row| !row.auth_implemented).collect();
        assert!(
            !unimplemented.is_empty(),
            "the shipped catalog contains on-demand-auth profiles; they must be visible"
        );
        for row in &unimplemented {
            assert!(!row.auth_implemented, "{} is marked unimplemented", row.id);
        }
    }

    #[test]
    fn show_renders_the_profile_detail() {
        let catalog = catalog();
        let text = show(&catalog, "openai").expect("openai");
        assert!(text.contains("openai"));
        assert!(text.contains("ApiKey"));
        assert!(text.contains("gpt-5"), "declared models are listed");
    }

    #[test]
    fn init_refuses_an_unimplemented_auth_kind() {
        let dir = tempfile::tempdir().expect("tempdir");
        let rows = profile_rows(&catalog());
        let Some(row) = rows.iter().find(|row| !row.auth_implemented) else {
            panic!("the shipped catalog has no unimplemented-auth profile to test with");
        };
        let error = init_selection(dir.path(), &row.id, "some-model").expect_err("refused");
        assert!(
            matches!(error, InitError::UnimplementedAuth(_, _)),
            "error: {error}"
        );
        assert!(
            orexis_composition::read_selection(dir.path()).is_none(),
            "a refused selection writes nothing"
        );
    }

    #[test]
    fn init_refuses_an_unknown_profile() {
        let dir = tempfile::tempdir().expect("tempdir");
        let error = init_selection(dir.path(), "no-such-profile", "m").expect_err("refused");
        assert!(
            matches!(error, InitError::UnknownProfile(_)),
            "error: {error}"
        );
    }

    #[test]
    fn init_writes_a_valid_selection() {
        let dir = tempfile::tempdir().expect("tempdir");
        let text = init_selection(dir.path(), "openai", "gpt-5").expect("selected");
        assert!(text.contains("openai"));
        let selection = orexis_composition::read_selection(dir.path()).expect("written");
        assert_eq!(selection.profile_id, "openai");
        assert_eq!(selection.model_id, "gpt-5");
    }
}
