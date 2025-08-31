use crate::{
    model::{HDKey, Transaction},
    security_utils::{public_key_to_hex, sign_hash},
};
use ed25519_dalek::SigningKey;
use ed25519_dalek::VerifyingKey;

pub struct Wallet {
    master_hdkey: HDKey,
    current_index: u32,
}

impl Wallet {
    pub fn new(seed: &str) -> Self {
        // TODO: improve seed generation
        let hdkey = HDKey::new(seed.as_bytes());
        Wallet {
            master_hdkey: hdkey,
            current_index: 0,
        }
    }

    fn derive_child(&self, index: u32) -> HDKey {
        self.master_hdkey.derive_child(index)
    }

    pub fn derive_path(&self, path: &[u32]) -> HDKey {
        let mut node = self.master_hdkey.clone();
        for &i in path {
            node = node.derive_child(i);
        }
        node
    }

    pub fn generate_n_keys(&self, n: u32) -> Vec<HDKey> {
        let mut keys = Vec::with_capacity(n as usize);
        for i in 0..n {
            let child_hdkey = self.derive_path(&[111, 0, 0, 0, i]);
            keys.push(child_hdkey);
        }
        keys
    }

    pub fn get_new_receive_addr(&mut self) -> VerifyingKey {
        let child_hdkey = self.derive_child(self.current_index);
        child_hdkey.get_public_key()
    }

    pub fn send_tx(
        &mut self,
        dest_pk: VerifyingKey,
        amount: f64,
        message: Option<String>,
    ) -> Transaction {
        let child_hdkey = self.derive_child(self.current_index);
        self.current_index += 1;

        let mut tx = Transaction::new(
            amount,
            public_key_to_hex(&dest_pk),
            public_key_to_hex(&child_hdkey.get_public_key()),
            message,
        );
        let signature = sign_hash(
            &mut SigningKey::from_bytes(&child_hdkey.private_key),
            tx.to_string().as_bytes(),
        );
        tx.signature = hex::encode(signature.to_bytes());
        tx
    }
}
