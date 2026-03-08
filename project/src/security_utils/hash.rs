use primitive_types::U256;
use sha2::{Digest, Sha256, Sha512};

pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

pub fn sha512(data: &[u8]) -> [u8; 64] {
    let mut hasher = Sha512::new();
    hasher.update(data);
    hasher.finalize().into()
}

pub fn bytes_to_hex_string(bytes: &[u8]) -> String {
    hex::encode(bytes)
}

/// Returns true if the hash is less than the given target (proof of work check).
pub fn hash_meets_target(hash: &[u8; 32], target: &U256) -> bool {
    U256::from_big_endian(hash) < *target
}

pub fn generate_sk_chain_code_from_data(data: &[u8]) -> ([u8; 32], [u8; 32]) {
    let hash = sha512(data);
    let master_sk = <[u8; 32]>::try_from(&hash[..32]).expect("slice with incorrect length");
    let chain_code = <[u8; 32]>::try_from(&hash[32..]).expect("slice with incorrect length");
    (master_sk, chain_code)
}
