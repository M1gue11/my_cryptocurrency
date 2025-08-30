use crate::security_utils::generate_sk_chain_code_from_data;
use ed25519_dalek::VerifyingKey;
use ed25519_dalek::{SecretKey, SigningKey};

pub struct HDKey {
    pub private_key: SecretKey,
    pub chain_code: [u8; 32],
}

impl HDKey {
    pub fn new(data: &[u8]) -> Self {
        let (private_key, chain_code) = generate_sk_chain_code_from_data(data);
        HDKey {
            private_key,
            chain_code,
        }
    }

    pub fn get_public_key(&self) -> VerifyingKey {
        let sk = SigningKey::from_bytes(&self.private_key);
        sk.verifying_key()
    }
}
