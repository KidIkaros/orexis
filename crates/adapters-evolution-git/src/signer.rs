//! Ed25519 signer.

use ed25519_dalek::{Signature, Signer as DalekSigner, SigningKey, Verifier};
use orexis_ports::{PortError, PortResult, Signer};

pub struct Ed25519Signer {
    key: SigningKey,
}

impl Ed25519Signer {
    pub fn from_seed(seed: [u8; 32]) -> Self {
        Self {
            key: SigningKey::from_bytes(&seed),
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> PortResult<Self> {
        let seed: [u8; 32] = bytes
            .try_into()
            .map_err(|_| PortError("ed25519 seed must be 32 bytes".into()))?;
        Ok(Self::from_seed(seed))
    }
}

impl Signer for Ed25519Signer {
    fn sign(&self, message: &[u8]) -> PortResult<Vec<u8>> {
        Ok(self.key.sign(message).to_bytes().to_vec())
    }

    fn verify(&self, message: &[u8], signature: &[u8]) -> bool {
        Signature::from_slice(signature)
            .map(|signature| self.key.verifying_key().verify(message, &signature).is_ok())
            .unwrap_or(false)
    }

    fn public_key(&self) -> Vec<u8> {
        self.key.verifying_key().to_bytes().to_vec()
    }
}
