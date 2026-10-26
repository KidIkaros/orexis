//! Provider profiles, the vendored catalog, and secret resolution (ADR-0002).

mod catalog;
mod profile;
mod secrets;

pub use catalog::{Catalog, CatalogError, ModelResolution};
pub use profile::*;
pub use secrets::{
    EnvSecrets, KeyringBackend, KeyringSecrets, MemorySecrets, SecretStore, SecretToolBackend,
};
