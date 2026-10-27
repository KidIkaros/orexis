//! Structural dry run over every shipped profile.

use std::path::PathBuf;

use orexis_adapters_provider_catalog::*;

#[test]
fn every_shipped_profile_is_structurally_usable() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("providers");
    let catalog = Catalog::load(&[dir]).expect("catalog");

    let mut openai = 0;
    let mut anthropic = 0;
    let mut gemini = 0;
    let mut llamacpp = 0;

    for profile in catalog.list() {
        // http(s) is the provider transport; acp:// is a peer reference
        // hermes models as a provider — loaded as data, refused at
        // composition.
        assert!(
            profile.base_url.starts_with("http") || profile.base_url.starts_with("acp://"),
            "{}: {}",
            profile.id,
            profile.base_url
        );
        assert!(
            !profile.base_url.contains('{'),
            "{}: placeholder base URL",
            profile.id
        );
        assert!(!profile.display_name.is_empty(), "{}", profile.id);
        assert!(
            profile.limits.max_concurrency >= 1,
            "{}: bad concurrency",
            profile.id
        );

        match profile.kind {
            ProviderKind::OpenAiCompatible => {
                openai += 1;
                assert_eq!(
                    profile.wire.dialect,
                    Dialect::OpenAi,
                    "{}: wrong dialect",
                    profile.id
                );
            }
            ProviderKind::Anthropic => {
                anthropic += 1;
                assert_eq!(
                    profile.wire.dialect,
                    Dialect::Anthropic,
                    "{}: wrong dialect",
                    profile.id
                );
            }
            ProviderKind::Gemini => gemini += 1,
            ProviderKind::LlamaCpp => llamacpp += 1,
            ProviderKind::Bedrock => {}
            ProviderKind::CodexResponses => {}
        }

        if let Some(model) = profile.models.first() {
            let resolved = catalog
                .resolve_model(&profile.id, &model.id)
                .expect("resolve");
            assert!(resolved.known, "{} {}", profile.id, model.id);
        }
    }

    assert!(openai > 150, "openai-compatible profiles: {openai}");
    assert!(anthropic >= 1, "anthropic profiles: {anthropic}");
    println!("openai={openai} anthropic={anthropic} gemini={gemini} llamacpp={llamacpp}");
}
