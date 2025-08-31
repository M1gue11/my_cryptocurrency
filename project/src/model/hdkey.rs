use crate::security_utils::generate_sk_chain_code_from_data;
use ed25519_dalek::VerifyingKey;
use ed25519_dalek::{SecretKey, SigningKey};
use std::fmt;

#[derive(Debug, Clone)]
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

    pub fn derive_child(&self, index: u32) -> HDKey {
        let mut data = Vec::with_capacity(1 + 32 + 4 + 32);
        data.push(0x00);
        data.extend_from_slice(&self.private_key);
        data.extend_from_slice(&index.to_be_bytes());
        data.extend_from_slice(&self.chain_code);

        HDKey::new(&data)
    }

    pub fn get_public_key(&self) -> VerifyingKey {
        let sk = SigningKey::from_bytes(&self.private_key);
        sk.verifying_key()
    }
}

impl fmt::Display for HDKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let private_key_hex = hex::encode(&self.private_key);
        let public_key_hex = hex::encode(self.get_public_key().as_bytes());
        write!(
            f,
            "Private Key: {} | Public Key: {}",
            private_key_hex, public_key_hex
        )
    }
}
