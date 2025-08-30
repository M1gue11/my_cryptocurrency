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

    pub fn derive_child(&self, index: u32) -> HDKey {
        // 0x00 is arbitrary
        // 0x00 || parent_sk || index_be
        let mut data = Vec::with_capacity(1 + 32 + 4 + 32);
        data.push(0x00);
        data.extend_from_slice(&self.master_hdkey.private_key);
        data.extend_from_slice(&index.to_be_bytes());
        data.extend_from_slice(&self.master_hdkey.chain_code);

        HDKey::new(&data)
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
