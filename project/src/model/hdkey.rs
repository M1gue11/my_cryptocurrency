use crate::security_utils::{generate_sk_chain_code_from_data, sha256, sign_hash};
use bs58;
use ed25519_dalek::{SecretKey, SigningKey};
use ed25519_dalek::{Signature, VerifyingKey};
use ripemd::Ripemd160;
use sha2::Digest;
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

    pub fn validate_address(address: &str) -> bool {
        let decoded = match bs58::decode(address).into_vec() {
            Ok(vec) => vec,
            Err(_) => return false,
        };

        if decoded.len() != 26 || decoded[0] != 0x00 || decoded[1] != 0x00 {
            return false;
        }

        let (body, checksum) = decoded.split_at(22);
        let calculated_checksum = &sha256(body)[..4];
        checksum == calculated_checksum
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

    fn get_address_impl(public_key: &VerifyingKey) -> String {
        let digest_1 = sha256(public_key.as_bytes());
        let ripemd = Ripemd160::digest(digest_1);
        let mut raw_addr = Vec::with_capacity(2 + ripemd.len() + 4);
        raw_addr.extend_from_slice(&[0x00, 0x00]);
        raw_addr.extend_from_slice(&ripemd);

        let checksum = &sha256(&raw_addr)[..4];
        let mut address = raw_addr;
        address.extend_from_slice(checksum);
        bs58::encode(address).into_string()
    }

    pub fn get_address(&self) -> String {
        HDKey::get_address_impl(&self.get_public_key())
    }

    pub fn sign(&self, message: &[u8]) -> Signature {
        sign_hash(&mut SigningKey::from_bytes(&self.private_key), message)
    }
}

impl fmt::Display for HDKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let private_key_hex = hex::encode(&self.private_key);
        let public_key_hex = hex::encode(self.get_public_key().as_bytes());
        write!(
            f,
            "Private Key: {} | Public Key: {} | Address: {}",
            private_key_hex,
            public_key_hex,
            self.get_address()
        )
    }
}
