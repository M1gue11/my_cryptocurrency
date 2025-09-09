use ed25519_dalek::Signature;
use ed25519_dalek::SigningKey;
use ed25519_dalek::Verifier;
use ed25519_dalek::VerifyingKey;
use ed25519_dalek::ed25519::signature::SignerMut;

use crate::security_utils::sha256;

pub fn public_key_to_hex(public_key: &VerifyingKey) -> String {
    hex::encode(public_key.to_bytes())
}

pub fn sign_hash(signing_key: &mut SigningKey, message: &[u8]) -> Signature {
    let hash = sha256(message);
    signing_key.try_sign(&hash).expect("Failed to sign message")
}

pub fn verify_signature(public_key: &VerifyingKey, message: &[u8], signature: Signature) -> bool {
    let hash = sha256(message);
    public_key.verify(&hash, &signature).is_ok()
}

pub fn load_signature_from_hex(hex: &str) -> Signature {
    let bytes = hex::decode(hex).expect("Failed to decode hex");
    let arr: [u8; 64] = bytes.try_into().expect("Signature must be 64 bytes");
    Signature::from_bytes(&arr)
}

pub fn load_public_key_from_hex(hex: &String) -> VerifyingKey {
    let bytes = hex::decode(hex).expect("Failed to decode hex");
    let arr: [u8; 32] = bytes.try_into().expect("Public key must be 32 bytes");
    VerifyingKey::from_bytes(&arr).expect("Failed to create public key from bytes")
}
