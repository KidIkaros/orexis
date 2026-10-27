//! Minisign signature verification — verify-only, hashed format
//! (ticket 021). The release pipeline (`scripts/release.sh`) produces
//! hashed signatures with `minisign` or `rsign`; this module verifies
//! them without either tool installed, on `ed25519-dalek` + BLAKE2b-512.
//!
//! Format (jedisct1's minisign): four lines — an untrusted comment
//! (covered by nothing), the base64 signature blob (`ED` + key id + the
//! ed25519 signature of the BLAKE2b-512 digest of the file), the trusted
//! comment (covered by the global signature), and the base64 global
//! signature (`ed25519(signature || trusted comment)`). Legacy `Ed`
//! signatures are refused: the pipeline never produces them, and the
//! hashed format is the modern requirement.

use base64::Engine;
use blake2::Digest;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MinisignError {
    #[error("malformed public key: {0}")]
    PublicKey(String),
    #[error("malformed signature: {0}")]
    Signature(String),
    #[error("legacy unhashed signatures are refused")]
    Legacy,
    #[error("the signature was made by another key")]
    WrongKey,
    #[error("signature verification failed")]
    Invalid,
}

/// A minisign public key, parsed from its two-line file form.
#[derive(Debug, Clone)]
pub struct PublicKey {
    key: VerifyingKey,
    key_id: [u8; 8],
}

impl PublicKey {
    /// Parse the public key file: an untrusted comment line, then the
    /// base64 key blob (`Ed` + key id + the ed25519 public key).
    pub fn parse(text: &str) -> Result<Self, MinisignError> {
        let blob =
            base64_line(text).ok_or_else(|| MinisignError::PublicKey("no key line".into()))?;
        let raw = base64::engine::general_purpose::STANDARD
            .decode(blob.trim())
            .map_err(|error| MinisignError::PublicKey(error.to_string()))?;
        if raw.len() != 2 + 8 + 32 {
            return Err(MinisignError::PublicKey(format!(
                "the key blob is {} bytes, expected 42",
                raw.len()
            )));
        }
        if &raw[..2] != b"Ed" {
            return Err(MinisignError::PublicKey(format!(
                "unknown key algorithm {:?}",
                String::from_utf8_lossy(&raw[..2])
            )));
        }
        let mut key_id = [0u8; 8];
        key_id.copy_from_slice(&raw[2..10]);
        let key = VerifyingKey::from_bytes(
            raw[10..42]
                .try_into()
                .map_err(|_| MinisignError::PublicKey("key slice".into()))?,
        )
        .map_err(|error| MinisignError::PublicKey(error.to_string()))?;
        Ok(Self { key, key_id })
    }
}

/// The second line of a minisign file — the base64 body.
fn base64_line(text: &str) -> Option<&str> {
    text.lines().filter(|line| !line.trim().is_empty()).nth(1)
}

/// Verify a file's bytes against its `.minisig` content.
pub fn verify(public_key: &PublicKey, file: &[u8], sig_text: &str) -> Result<(), MinisignError> {
    let mut lines = sig_text.lines().filter(|line| !line.trim().is_empty());
    // The untrusted comment is covered by nothing; it is never a trust
    // decision.
    let _untrusted = lines
        .next()
        .ok_or_else(|| MinisignError::Signature("empty".into()))?;
    let blob_line = lines
        .next()
        .ok_or_else(|| MinisignError::Signature("no blob".into()))?;
    let trusted_line = lines
        .next()
        .ok_or_else(|| MinisignError::Signature("no trusted comment".into()))?;
    let global_line = lines
        .next()
        .ok_or_else(|| MinisignError::Signature("no global signature".into()))?;

    let blob = base64::engine::general_purpose::STANDARD
        .decode(blob_line.trim())
        .map_err(|error| MinisignError::Signature(error.to_string()))?;
    if blob.len() != 2 + 8 + 64 {
        return Err(MinisignError::Signature(format!(
            "the signature blob is {} bytes, expected 74",
            blob.len()
        )));
    }
    if &blob[..2] != b"ED" {
        if &blob[..2] == b"Ed" {
            return Err(MinisignError::Legacy);
        }
        return Err(MinisignError::Signature(format!(
            "unknown signature algorithm {:?}",
            String::from_utf8_lossy(&blob[..2])
        )));
    }
    if blob[2..10] != public_key.key_id {
        return Err(MinisignError::WrongKey);
    }
    let signature = Signature::from_slice(&blob[10..74])
        .map_err(|error| MinisignError::Signature(error.to_string()))?;

    // The hashed format signs the BLAKE2b-512 digest of the file.
    let digest = blake2::Blake2b512::digest(file);
    public_key
        .key
        .verify(&digest, &signature)
        .map_err(|_| MinisignError::Invalid)?;

    // The global signature covers the signature and the trusted comment
    // (everything after the `trusted comment: ` prefix, no newline).
    let trusted = trusted_line
        .strip_prefix("trusted comment: ")
        .ok_or_else(|| MinisignError::Signature("the trusted comment line is malformed".into()))?;
    let global = base64::engine::general_purpose::STANDARD
        .decode(global_line.trim())
        .map_err(|error| MinisignError::Signature(error.to_string()))?;
    let global = Signature::from_slice(&global)
        .map_err(|error| MinisignError::Signature(error.to_string()))?;
    let mut signed = Vec::with_capacity(64 + trusted.len());
    signed.extend_from_slice(&blob[10..74]);
    signed.extend_from_slice(trusted.as_bytes());
    public_key
        .key
        .verify(&signed, &global)
        .map_err(|_| MinisignError::Invalid)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const PUB: &str = include_str!("../test-fixtures/test.pub");
    const FIXTURE: &[u8] = b"orexis test fixture\n";
    const SIG: &str = include_str!("../test-fixtures/fixture.txt.minisig");

    #[test]
    fn a_reference_signature_verifies() {
        // The fixture was produced by rsign (the minisign reference
        // implementation family); interop is the format's proof.
        let key = PublicKey::parse(PUB).expect("key");
        verify(&key, FIXTURE, SIG).expect("verifies");
    }

    #[test]
    fn a_tampered_file_is_refused() {
        let key = PublicKey::parse(PUB).expect("key");
        let error = verify(&key, b"orexis test fixture tampered\n", SIG).expect_err("refused");
        assert_eq!(error, MinisignError::Invalid);
    }

    #[test]
    fn a_tampered_trusted_comment_is_refused() {
        // The trusted comment is covered by the global signature.
        let key = PublicKey::parse(PUB).expect("key");
        let tampered = SIG.replace("fixture: test file", "fixture: EVIL");
        let error = verify(&key, FIXTURE, &tampered).expect_err("refused");
        assert_eq!(error, MinisignError::Invalid);
    }

    #[test]
    fn another_key_is_refused() {
        // A signature made by another key must not verify: the key id
        // check fires before any signature math.
        let key = PublicKey::parse(PUB).expect("key");
        let other_sig = include_str!("../test-fixtures/other-key.minisig");
        let error = verify(&key, FIXTURE, other_sig).expect_err("refused");
        assert_eq!(error, MinisignError::WrongKey);
    }

    #[test]
    fn a_legacy_signature_is_refused() {
        // The pipeline produces hashed ("ED") signatures only; a legacy
        // ("Ed") tag is refused before any signature math.
        let key = PublicKey::parse(PUB).expect("key");
        use base64::Engine;
        let mut lines = SIG.lines();
        let untrusted = lines.next().expect("untrusted");
        let blob_line = lines.next().expect("blob");
        let mut blob = base64::engine::general_purpose::STANDARD
            .decode(blob_line.trim())
            .expect("blob");
        blob[..2].copy_from_slice(b"Ed");
        let legacy_blob = base64::engine::general_purpose::STANDARD.encode(&blob);
        let rest = &SIG[SIG.find("trusted").expect("trusted line")..];
        let legacy = format!("{untrusted}\n{legacy_blob}\n{rest}");
        let error = verify(&key, FIXTURE, &legacy).expect_err("refused");
        assert_eq!(error, MinisignError::Legacy);
    }

    #[test]
    fn a_truncated_signature_is_refused() {
        let key = PublicKey::parse(PUB).expect("key");
        let truncated = SIG.lines().take(3).collect::<Vec<_>>().join("\n");
        let error = verify(&key, FIXTURE, &truncated).expect_err("refused");
        assert!(
            matches!(error, MinisignError::Signature(_)),
            "error: {error}"
        );
    }
}
