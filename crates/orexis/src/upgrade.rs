//! `orexis upgrade` (ADR-0019, ticket 021): fetch the release manifest,
//! verify its signature against the pinned key, download the artifact,
//! verify its checksum, and replace the binary. State is never touched —
//! sessions and the entity live outside the binary.

use std::path::{Path, PathBuf};

use orexis_adapters_http::Transport;

use crate::minisign::{self, PublicKey};

#[derive(Debug, thiserror::Error)]
pub enum UpgradeError {
    #[error("the release public key is not pinned yet; see crates/orexis/orexis.pub (ticket 020's wizard step)")]
    KeyNotPinned,
    #[error("minisign: {0}")]
    Minisign(#[from] minisign::MinisignError),
    #[error("manifest: {0}")]
    Manifest(String),
    #[error("no artifact for this platform ({0}) in the manifest")]
    NoArtifact(String),
    #[error("already at the newest version ({0})")]
    UpToDate(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("transport: {0}")]
    Transport(String),
}

/// The host's release target triple.
pub fn host_target() -> Result<&'static str, UpgradeError> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => Ok("x86_64-unknown-linux-gnu"),
        ("linux", "aarch64") => Ok("aarch64-unknown-linux-gnu"),
        ("macos", "x86_64") => Ok("x86_64-apple-darwin"),
        ("macos", "aarch64") => Ok("aarch64-apple-darwin"),
        (os, arch) => Err(UpgradeError::NoArtifact(format!("{os}-{arch}"))),
    }
}

/// Compare dotted versions numerically; missing components are zero, so
/// `0.1` equals `0.1.0`.
pub fn compare_versions(left: &str, right: &str) -> std::cmp::Ordering {
    let left: Vec<u64> = left
        .split('.')
        .map(|part| part.parse().unwrap_or(0))
        .collect();
    let right: Vec<u64> = right
        .split('.')
        .map(|part| part.parse().unwrap_or(0))
        .collect();
    let len = left.len().max(right.len());
    for index in 0..len {
        let (a, b) = (
            left.get(index).copied().unwrap_or(0),
            right.get(index).copied().unwrap_or(0),
        );
        if a != b {
            return a.cmp(&b);
        }
    }
    std::cmp::Ordering::Equal
}

/// One artifact entry of the manifest.
#[derive(Debug)]
struct Artifact {
    version: String,
    url: String,
    sha256: String,
}

/// Read the manifest, verifying its signature against `pinned`.
fn read_manifest_with(
    pinned: &str,
    manifest: &str,
    sig: &str,
) -> Result<Vec<Artifact>, UpgradeError> {
    let key = PublicKey::parse(pinned).map_err(|_| UpgradeError::KeyNotPinned)?;
    minisign::verify(&key, manifest.as_bytes(), sig)?;

    let value: serde_json::Value = serde_json::from_str(manifest)
        .map_err(|error| UpgradeError::Manifest(error.to_string()))?;
    let version = value
        .get("version")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| UpgradeError::Manifest("no version".into()))?
        .to_string();
    let target = host_target()?;
    let artifacts = value
        .get("artifacts")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| UpgradeError::Manifest("no artifacts".into()))?
        .iter()
        .filter_map(|artifact| {
            if artifact.get("target")?.as_str()? != target {
                return None;
            }
            Some(Artifact {
                version: version.clone(),
                url: artifact.get("url")?.as_str()?.to_string(),
                sha256: artifact.get("sha256")?.as_str()?.to_string(),
            })
        })
        .collect::<Vec<_>>();
    if artifacts.is_empty() {
        return Err(UpgradeError::NoArtifact(target.to_string()));
    }
    Ok(artifacts)
}

/// The pinned release key; the wizard step (ticket 020) replaces the
/// placeholder's content.
const PINNED_KEY: &str = include_str!("../orexis.pub");

/// The upgrade. `from` sources the manifest and artifact from a local
/// directory (offline installs, and how this is tested); `None` fetches
/// from the release repo named by `OREXIS_RELEASE_REPO`.
pub fn upgrade(from: Option<&Path>) -> Result<String, UpgradeError> {
    let (artifacts, artifact_bytes) = match from {
        Some(dir) => {
            let manifest = std::fs::read_to_string(dir.join("latest.json"))?;
            let sig = std::fs::read_to_string(dir.join("latest.json.minisig"))?;
            let artifacts = read_manifest_with(PINNED_KEY, &manifest, &sig)?;
            let name = artifact_name(&artifacts[0].url)?;
            (artifacts, std::fs::read(dir.join(&name))?)
        }
        None => {
            let repo = std::env::var("OREXIS_RELEASE_REPO").map_err(|_| {
                UpgradeError::Manifest(
                    "OREXIS_RELEASE_REPO is not set (owner/repo); the upgrade does not guess"
                        .into(),
                )
            })?;
            let base = format!("https://github.com/{repo}/releases/latest/download");
            let transport = orexis_adapters_http::ReqwestTransport::new();
            let manifest = fetch_text(&transport, &format!("{base}/latest.json"))?;
            let sig = fetch_text(&transport, &format!("{base}/latest.json.minisig"))?;
            let artifacts = read_manifest_with(PINNED_KEY, &manifest, &sig)?;
            let bytes = fetch_bytes(
                &transport,
                &format!("{base}/{}", artifact_name(&artifacts[0].url)?),
            )?;
            (artifacts, bytes)
        }
    };
    let artifact = &artifacts[0];

    let current = env!("CARGO_PKG_VERSION");
    if compare_versions(&artifact.version, current) != std::cmp::Ordering::Greater {
        return Err(UpgradeError::UpToDate(format!(
            "v{current} (the manifest offers v{})",
            artifact.version
        )));
    }

    verify_checksum(&artifact_bytes, &artifact.sha256)?;

    // The artifact is a tar.gz carrying the `orexis` binary; extract and
    // replace. Extraction shells to the system `tar` — the installer
    // requires it too, and pulling tar+flate2 into the binary for one
    // call is not worth the surface.
    let work = std::env::temp_dir().join(format!("orexis-upgrade-{}", std::process::id()));
    let extracted = extract_binary(&artifact_bytes, &work)?;
    let bytes = std::fs::read(&extracted)?;
    let exe = std::env::current_exe()?;
    let replaced = replace_binary_at(&exe, &bytes)?;
    let _ = std::fs::remove_dir_all(&work);
    Ok(format!(
        "upgraded v{current} -> v{}; state untouched\n{replaced}",
        artifact.version
    ))
}

/// The artifact's file name, from its manifest url.
fn artifact_name(url: &str) -> Result<String, UpgradeError> {
    url.rsplit('/')
        .next()
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .ok_or_else(|| UpgradeError::Manifest("artifact url".into()))
}

async fn fetch_response(
    transport: &orexis_adapters_http::ReqwestTransport,
    url: &str,
) -> Result<Vec<u8>, UpgradeError> {
    let response = transport
        .get_response(url, Vec::new())
        .await
        .map_err(|error| UpgradeError::Transport(error.to_string()))?;
    if !response.is_success() {
        return Err(UpgradeError::Transport(format!(
            "the release answered HTTP {}",
            response.status
        )));
    }
    let mut body = Vec::new();
    let mut stream = response.body;
    while let Some(chunk) = futures::StreamExt::next(&mut stream).await {
        let chunk = chunk.map_err(|error| UpgradeError::Transport(error.to_string()))?;
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

fn fetch_text(
    transport: &orexis_adapters_http::ReqwestTransport,
    url: &str,
) -> Result<String, UpgradeError> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let bytes = runtime.block_on(fetch_response(transport, url))?;
    Ok(String::from_utf8_lossy(&bytes).to_string())
}

fn fetch_bytes(
    transport: &orexis_adapters_http::ReqwestTransport,
    url: &str,
) -> Result<Vec<u8>, UpgradeError> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(fetch_response(transport, url))
}

/// sha256 against the manifest's value.
fn verify_checksum(bytes: &[u8], expected: &str) -> Result<(), UpgradeError> {
    use sha2::Digest;
    let actual = format!("{:x}", sha2::Sha256::digest(bytes));
    if actual != expected.to_ascii_lowercase() {
        return Err(UpgradeError::Manifest(format!(
            "checksum mismatch: expected {expected}, got {actual}"
        )));
    }
    Ok(())
}

/// Replace the binary at `path` with `bytes`: a temp file in the same
/// directory (same filesystem, so the rename is atomic), then rename
/// over it. The running process keeps executing the old inode; the next
/// launch runs the new binary.
pub fn replace_binary_at(path: &Path, bytes: &[u8]) -> Result<String, UpgradeError> {
    let dir = path
        .parent()
        .ok_or_else(|| UpgradeError::Io(std::io::Error::other("the binary has no parent")))?;
    let temp = dir.join(format!(".orexis-upgrade-{}", std::process::id()));
    std::fs::write(&temp, bytes)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&temp, std::fs::Permissions::from_mode(0o755))?;
    }
    std::fs::rename(&temp, path).map_err(|error| {
        let _ = std::fs::remove_file(&temp);
        UpgradeError::Io(error)
    })?;
    Ok(format!("replaced {}", path.display()))
}

/// Extract the `orexis` binary from a tar.gz artifact into `dir`.
pub fn extract_binary(artifact: &[u8], dir: &Path) -> Result<PathBuf, UpgradeError> {
    std::fs::create_dir_all(dir)?;
    let archive = dir.join("artifact.tar.gz");
    std::fs::write(&archive, artifact)?;
    let status = std::process::Command::new("tar")
        .arg("-xzf")
        .arg(&archive)
        .current_dir(dir)
        .status()?;
    if !status.success() {
        return Err(UpgradeError::Manifest(format!("tar exited with {status}")));
    }
    let binary = dir.join("orexis");
    if !binary.exists() {
        return Err(UpgradeError::Manifest(
            "the artifact carries no orexis binary".into(),
        ));
    }
    Ok(binary)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minisign keypair and signer built on dalek, for fixtures: the
    /// same format the reference tools produce, generated in-test so no
    /// secret material is committed.
    struct TestSigner {
        verifying: VerifyingKey,
        signing: ed25519_dalek::SigningKey,
        key_id: [u8; 8],
    }

    impl TestSigner {
        fn new(seed: u64) -> Self {
            let mut seed32 = [0u8; 32];
            seed32[..8].copy_from_slice(&seed.to_le_bytes());
            let signing = ed25519_dalek::SigningKey::from_bytes(&seed32);
            let mut key_id = [0u8; 8];
            key_id.copy_from_slice(&signing.verifying_key().to_bytes()[..8]);
            Self {
                verifying: signing.verifying_key(),
                signing,
                key_id,
            }
        }

        /// The two-line public key file.
        fn public_key_file(&self) -> String {
            use base64::Engine as _;

            let mut blob = Vec::new();
            blob.extend_from_slice(b"Ed");
            blob.extend_from_slice(&self.key_id);
            blob.extend_from_slice(&self.verifying.to_bytes());
            format!(
                "untrusted comment: test key\n{}\n",
                base64::engine::general_purpose::STANDARD.encode(&blob)
            )
        }

        /// The four-line signature file for `data`.
        fn sign(&self, data: &[u8], trusted: &str) -> String {
            use base64::Engine as _;
            use blake2::Digest;
            use ed25519_dalek::Signer;
            let digest = blake2::Blake2b512::digest(data);
            let signature = self.signing.sign(&digest);
            let mut blob = Vec::new();
            blob.extend_from_slice(b"ED");
            blob.extend_from_slice(&self.key_id);
            blob.extend_from_slice(&signature.to_bytes());
            let mut signed = Vec::new();
            signed.extend_from_slice(&signature.to_bytes());
            signed.extend_from_slice(trusted.as_bytes());
            let global = self.signing.sign(&signed);
            format!(
                "untrusted comment: test signature\n{}\ntrusted comment: {trusted}\n{}\n",
                base64::engine::general_purpose::STANDARD.encode(&blob),
                base64::engine::general_purpose::STANDARD.encode(global.to_bytes())
            )
        }
    }

    use ed25519_dalek::VerifyingKey;

    #[test]
    fn versions_compare_numerically() {
        use std::cmp::Ordering;
        assert_eq!(compare_versions("0.1.0", "0.2.0"), Ordering::Less);
        assert_eq!(compare_versions("0.10.0", "0.9.0"), Ordering::Greater);
        assert_eq!(compare_versions("1.0.0", "1.0.0"), Ordering::Equal);
        assert_eq!(compare_versions("0.1", "0.1.0"), Ordering::Equal);
    }

    #[test]
    fn the_host_target_maps_the_platform() {
        let target = host_target().expect("target");
        assert!(
            [
                "x86_64-unknown-linux-gnu",
                "aarch64-unknown-linux-gnu",
                "x86_64-apple-darwin",
                "aarch64-apple-darwin"
            ]
            .contains(&target),
            "target: {target}"
        );
    }

    #[test]
    fn an_unpinned_key_is_a_clear_error() {
        // The placeholder orexis.pub names the wizard step; parsing it
        // must fail with the key-not-pinned error, not a cryptic parse
        // error.
        let error = read_manifest_with(PINNED_KEY, "{}", "sig").expect_err("refused");
        assert!(
            matches!(error, UpgradeError::KeyNotPinned),
            "error: {error}"
        );
    }

    #[test]
    fn a_tampered_manifest_is_refused_by_the_signature() {
        let signer = TestSigner::new(7);
        let manifest = r#"{"version":"9.9.9","artifacts":[]}"#;
        let sig = signer.sign(manifest.as_bytes(), "test");
        let tampered = manifest.replace("9.9.9", "9.9.10");
        let error =
            read_manifest_with(&signer.public_key_file(), &tampered, &sig).expect_err("refused");
        assert!(matches!(error, UpgradeError::Minisign(_)), "error: {error}");
    }

    #[test]
    fn the_upgrade_replaces_the_binary_and_touches_nothing_else() {
        // The state-preservation proof, through the full verified path:
        // a signed manifest, a checksummed artifact, files around the
        // binary surviving byte-identical.
        let dir = tempfile::tempdir().expect("tempdir");
        let bin = dir.path().join("orexis");
        std::fs::write(&bin, b"old binary").expect("old");
        let state = dir.path().join("state.db");
        std::fs::write(&state, b"state bytes").expect("state");

        // The artifact: a tar.gz carrying the "new" binary.
        let artifact_dir = dir.path().join("art");
        std::fs::create_dir_all(&artifact_dir).expect("art");
        let stage = dir.path().join("stage");
        std::fs::create_dir_all(&stage).expect("stage");
        std::fs::write(stage.join("orexis"), b"new binary").expect("new");
        let status = std::process::Command::new("tar")
            .arg("-czf")
            .arg(artifact_dir.join("orexis-9.9.9-x86_64-unknown-linux-gnu.tar.gz"))
            .arg("-C")
            .arg(&stage)
            .arg(".")
            .status()
            .expect("tar");
        assert!(status.success(), "tar failed");

        // The manifest, signed with the in-test keypair.
        let signer = TestSigner::new(11);
        use sha2::Digest;
        let artifact_bytes =
            std::fs::read(artifact_dir.join("orexis-9.9.9-x86_64-unknown-linux-gnu.tar.gz"))
                .expect("artifact");
        let manifest = serde_json::json!({
            "version": "9.9.9",
            "artifacts": [{
                "target": host_target().expect("target"),
                "url": "https://github.com/example/orexis/releases/download/v9.9.9/orexis-9.9.9-x86_64-unknown-linux-gnu.tar.gz",
                "sha256": format!("{:x}", sha2::Sha256::digest(&artifact_bytes)),
            }]
        });
        let manifest_text = manifest.to_string();
        let sig = signer.sign(manifest_text.as_bytes(), "test manifest");

        // The full verified read: signature, then checksum, then the
        // replace.
        let artifacts =
            read_manifest_with(&signer.public_key_file(), &manifest_text, &sig).expect("manifest");
        assert_eq!(artifacts[0].version, "9.9.9");
        verify_checksum(&artifact_bytes, &artifacts[0].sha256).expect("checksum");
        let extracted = extract_binary(&artifact_bytes, &dir.path().join("work")).expect("extract");
        let replaced = replace_binary_at(&bin, &std::fs::read(&extracted).expect("new bytes"))
            .expect("replace");
        assert!(replaced.contains("orexis"));

        // The binary is the artifact's; the state is untouched.
        assert_eq!(std::fs::read(&bin).expect("new"), b"new binary");
        assert_eq!(std::fs::read(&state).expect("state"), b"state bytes");
    }
}
