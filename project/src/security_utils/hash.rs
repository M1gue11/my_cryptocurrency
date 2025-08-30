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

pub fn digest_to_hex_string(digest: &[u8; 32]) -> String {
    digest.iter().map(|b| format!("{:02x}", b)).collect()
}

pub fn hash_starts_with_zero_bits(hash: &[u8; 32], n: usize) -> bool {
    let full_bytes = n / 8;
    let remaining_bits = n % 8;

    for i in 0..full_bytes {
        if hash[i] != 0 {
            return false;
        }
    }

    if remaining_bits > 0 {
        let mask = 0xFF << (8 - remaining_bits);
        if hash[full_bytes] & mask != 0 {
            return false;
        }
    }
    true
}

pub fn generate_sk_chain_code_from_data(data: &[u8]) -> ([u8; 32], [u8; 32]) {
    let hash = sha512(data);
    let master_sk = <[u8; 32]>::try_from(&hash[..32]).expect("slice with incorrect length");
    let chain_code = <[u8; 32]>::try_from(&hash[32..]).expect("slice with incorrect length");
    (master_sk, chain_code)
}
