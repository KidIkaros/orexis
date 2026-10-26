//! EXPERIMENTAL hybrid signer (Ed25519 + Falcon-1024) over the `Signer` port.
//!
//! Not a workspace member; not built by default. The SDK is explicitly
//! unaudited, so this must not become the default evolution signer before
//! M6-R01 completes and a new ADR accepts it.

use orexis_ports::{PortError, PortResult, Signer};
use origin_crypto_sdk::pqc::falcon1024::FalconSignature;
use origin_crypto_sdk::signing::hybrid::{Ed25519Falcon1024, HybridSigningKeyBundle};
use origin_crypto_sdk::Ed25519Signature;

pub struct OriginHybridSigner {
    bundle: HybridSigningKeyBundle,
}

impl OriginHybridSigner {
    pub fn from_seed(seed: [u8; 32], domain: &str) -> PortResult<Self> {
        let bundle = HybridSigningKeyBundle::from_seed(&seed, domain)
            .map_err(|error| PortError(format!("hybrid bundle: {error}")))?;
        Ok(Self { bundle })
    }
}

impl Signer for OriginHybridSigner {
    fn sign(&self, message: &[u8]) -> PortResult<Vec<u8>> {
        let signature = self
            .bundle
            .try_sign_hybrid(message)
            .map_err(|error| PortError(format!("hybrid sign: {error}")))?;
        let ed = signature.ed25519_sig.to_bytes();
        let falcon = signature.falcon_sig.as_bytes();
        let mut out = Vec::with_capacity(2 + ed.len() + falcon.len());
        out.extend_from_slice(&(falcon.len() as u16).to_be_bytes());
        out.extend_from_slice(&ed);
        out.extend_from_slice(falcon);
        Ok(out)
    }

    fn verify(&self, message: &[u8], signature: &[u8]) -> bool {
        let Some((ed, falcon)) = split(signature) else {
            return false;
        };
        let Ok(ed) = <[u8; 64]>::try_from(ed) else {
            return false;
        };
        let ed25519_sig = Ed25519Signature::from_bytes(&ed);
        let Ok(falcon_sig) = FalconSignature::from_bytes(falcon) else {
            return false;
        };
        let hybrid = Ed25519Falcon1024 {
            ed25519_sig,
            falcon_sig,
        };
        Ed25519Falcon1024::verify(
            self.bundle.ed25519_pk(),
            self.bundle.falcon1024_pk(),
            message,
            &hybrid,
        )
        .is_ok()
    }

    fn public_key(&self) -> Vec<u8> {
        let ed = self.bundle.ed25519_pk().to_bytes();
        let falcon = self.bundle.falcon1024_pk().as_bytes();
        let mut out = Vec::with_capacity(2 + ed.len() + falcon.len());
        out.extend_from_slice(&(falcon.len() as u16).to_be_bytes());
        out.extend_from_slice(&ed);
        out.extend_from_slice(falcon);
        out
    }
}

fn split(signature: &[u8]) -> Option<(&[u8], &[u8])> {
    if signature.len() < 2 + 64 {
        return None;
    }
    let len = u16::from_be_bytes([signature[0], signature[1]]) as usize;
    if signature.len() != 2 + 64 + len {
        return None;
    }
    Some((&signature[2..66], &signature[66..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hybrid_signs_and_verifies() {
        let signer = OriginHybridSigner::from_seed([9u8; 32], "orexis-evolution-test")
            .expect("signer");
        let signature = signer.sign(b"ledger entry").expect("sign");
        assert!(signer.verify(b"ledger entry", &signature));
        assert!(!signer.verify(b"tampered", &signature));
        assert!(!signer.public_key().is_empty());
    }

    #[test]
    fn malformed_signature_is_rejected() {
        let signer = OriginHybridSigner::from_seed([9u8; 32], "orexis-evolution-test")
            .expect("signer");
        assert!(!signer.verify(b"x", &[]));
        assert!(!signer.verify(b"x", &[0, 0]));
    }
}
