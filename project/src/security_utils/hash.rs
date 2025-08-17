use sha2::{Digest, Sha256};

pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
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
